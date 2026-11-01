//! `gamecraft-mcp`: hand-rolled MCP stdio server.
//!
//! Newline-delimited JSON-RPC on stdin/stdout; logs go to stderr so stdout stays
//! 100% JSON.
//!
//! Dual-era, as the spec allows:
//! - **Modern** `2026-07-28` (stateless): no handshake; every request carries
//!   `_meta["io.modelcontextprotocol/protocolVersion"]`. `server/discover`
//!   advertises versions and capabilities; results carry `resultType` and
//!   `_meta["io.modelcontextprotocol/serverInfo"]`; `tools/list` carries the
//!   `ttlMs`/`cacheScope` cache hints. An unsupported version gets
//!   `UnsupportedProtocolVersionError` (-32022) listing what is supported.
//! - **Legacy** `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05`: the
//!   `initialize` handshake, negotiated to the client's revision (older clients
//!   ignore the newer tool fields: titles, annotations, `outputSchema`).
//!
//! Tools list in a fixed order (cache-friendly), and input validation failures
//! are tool execution errors, not protocol errors, so the model can self-correct.
//!
//! Result contract (fleet "dialogic returns", spec-native form):
//! - success: `content[0]` is a one-sentence summary for the conversation,
//!   `content[1]` the JSON data, `structuredContent` the same data plus
//!   `success: true` and `message`.
//! - tool failure (bad input, missing file, gate disabled): `isError: true`,
//!   with `message`, a `suggestion` and a concrete `remediation`, so the model
//!   can fix the call instead of guessing. Only malformed requests and unknown
//!   tools are JSON-RPC errors.

use gamecraft_mcp::catalog::{self, GameKind};
use gamecraft_mcp::{scene_lint, spatial_query};
use serde_json::{json, Value};
use std::fmt::Write as _;
use std::io::{BufRead, Write};

/// Stateless revisions: version, identity and capabilities ride on every request.
const MODERN_VERSIONS: [&str; 1] = ["2026-07-28"];
/// Handshake revisions, newest first; `initialize` negotiates among these.
const LEGACY_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
/// `_meta` key carrying a modern request's protocol version.
const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
/// `_meta` key carrying the server's identity on modern results.
const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";
/// `UnsupportedProtocolVersionError` code (2026-07-28 allocation).
const UNSUPPORTED_VERSION: i32 = -32_022;
/// Cache hint for list and discover results: the tool set only changes with the binary.
const TTL_MS: u64 = 3_600_000;

/// Every revision this server speaks, newest first.
fn all_versions() -> Vec<&'static str> {
    MODERN_VERSIONS
        .iter()
        .chain(LEGACY_VERSIONS.iter())
        .copied()
        .collect()
}

/// Server identity (`Implementation`), with the 2025-11-25 `description`.
fn server_info() -> Value {
    json!({
        "name": "gamecraft-mcp",
        "title": "GameCraft",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Godot 4 scene tools and a verified catalog of Godot and three.js games."
    })
}

/// Usage notes sent to the client at `initialize`.
const INSTRUCTIONS: &str = "GameCraft: Godot 4 scene tools plus a catalog of playable games (forge-made Godot projects and three.js pages). \
Start with game_list to see what exists and how each game last verified; game_report gives one game's cheat sheet and gate record. \
scene_lint / scene_inspect / spatial_query read .tscn text; export_scene and game_create write games from JSON specs. \
Running engines or browsers is opt-in: scene_verify needs GAMECRAFT_MCP_ALLOW_GODOT=1, game_verify on web games needs GAMECRAFT_MCP_ALLOW_WEB=1. \
Every result starts with a one-line summary; failures carry a suggestion and a remediation.";

fn main() {
    // Any engine or browser this server launches dies with it, even if the client kills us mid-run.
    let _ = gamecraft_mcp::proc_guard::kill_children_on_exit();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(text) = line else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(&text) {
            let mut stdout = std::io::stdout();
            if writeln!(stdout, "{reply}").is_err() || stdout.flush().is_err() {
                break;
            }
        }
    }
}

/// Handle one JSON-RPC message; `None` = notification, no reply.
fn handle(text: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(err) => {
            let reply = json!({"jsonrpc": "2.0", "id": Value::Null, "error": {"code": -32700, "message": format!("parse error: {err}")}});
            return serde_json::to_string(&reply).ok();
        }
    };
    let method = msg.get("method")?.as_str()?.to_string();
    let id = msg.get("id").cloned()?;
    serde_json::to_string(&respond(method.as_str(), &msg, &id)).ok()
}

/// The version a modern request declares in `_meta`, if any.
fn requested_version(msg: &Value) -> Option<&str> {
    msg.pointer("/params/_meta")
        .and_then(|meta| meta.get(META_VERSION))
        .and_then(Value::as_str)
}

/// Build the reply for one request: version check, dispatch, modern stamping.
fn respond(method: &str, msg: &Value, id: &Value) -> Value {
    let requested = requested_version(msg);
    if let Some(version) = requested {
        if !all_versions().contains(&version) {
            return json!({"jsonrpc": "2.0", "id": id, "error": {
                "code": UNSUPPORTED_VERSION,
                "message": "Unsupported protocol version",
                "data": {"supported": all_versions(), "requested": version}
            }});
        }
    }
    let modern = requested.is_some_and(|v| MODERN_VERSIONS.contains(&v));
    match dispatch(method, msg, modern) {
        Ok(mut value) => {
            if modern || method == "server/discover" {
                stamp_modern(&mut value);
            }
            json!({"jsonrpc": "2.0", "id": id, "result": value})
        }
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    }
}

/// Modern results carry `resultType` and the server's identity in `_meta`.
fn stamp_modern(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("resultType".to_string(), json!("complete"));
        let meta = obj.entry("_meta").or_insert_with(|| json!({}));
        if let Some(meta) = meta.as_object_mut() {
            meta.insert(META_SERVER_INFO.to_string(), server_info());
        }
    }
}

/// Pick the legacy revision for `initialize`: the client's if we speak it, else
/// our newest handshake revision.
fn negotiate(msg: &Value) -> &'static str {
    let asked = msg
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("");
    LEGACY_VERSIONS
        .iter()
        .copied()
        .find(|v| *v == asked)
        .unwrap_or(LEGACY_VERSIONS[0])
}

/// Route one method call; `Err((code, message))` becomes a JSON-RPC error.
fn dispatch(method: &str, msg: &Value, modern: bool) -> Result<Value, (i32, String)> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": negotiate(msg),
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": server_info(),
            "instructions": INSTRUCTIONS,
        })),
        "server/discover" => Ok(json!({
            "supportedVersions": all_versions(),
            "capabilities": {"tools": {"listChanged": false}},
            "instructions": INSTRUCTIONS,
            "ttlMs": TTL_MS,
            "cacheScope": "public",
        })),
        "tools/list" if modern => Ok(json!({
            "tools": tool_list(),
            "ttlMs": TTL_MS,
            "cacheScope": "public",
        })),
        "tools/list" => Ok(json!({ "tools": tool_list() })),
        "tools/call" => {
            let params = msg
                .get("params")
                .ok_or_else(|| (-32602_i32, "missing params".to_string()))?;
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| (-32602_i32, "missing tool name".to_string()))?;
            let empty = json!({});
            let args = params.get("arguments").unwrap_or(&empty);
            call_tool(name, args)
        }
        "ping" => Ok(json!({})),
        _ => Err((-32601, format!("method not found: {method}"))),
    }
}

// ------------------------------------------------------------------ tool list

/// Output schema shared by every tool: the dialogic envelope, open for tool data.
fn envelope(props: &Value) -> Value {
    let mut properties = json!({
        "success": {"type": "boolean", "description": "true on success; false on a tool error (then see dialogic)."},
        "message": {"type": "string", "description": "One-sentence summary, safe to show the user."},
        "error": {"type": "string", "description": "Short error code, only on failure."},
        "dialogic": {"type": "object", "description": "On failure: suggestion (what to do) and remediation (a concrete next call)."}
    });
    if let (Some(base), Some(extra)) = (properties.as_object_mut(), props.as_object()) {
        for (k, v) in extra {
            base.insert(k.clone(), v.clone());
        }
    }
    json!({"type": "object", "properties": properties, "required": ["success", "message"]})
}

/// Behaviour hints. Every tool here is idempotent and touches only local files and processes.
fn annotations(read_only: bool, destructive: bool) -> Value {
    json!({"readOnlyHint": read_only, "destructiveHint": destructive, "idempotentHint": true, "openWorldHint": false})
}

const SCENE_SOURCE: &str = "Give the scene as scene_text (full .tscn/.tres source) or scene_file (path; res:// paths need root).";

#[allow(clippy::too_many_lines)] // one JSON entry per tool; the protocol surface reads best in one place
fn tool_list() -> Value {
    let scene_props = json!({
        "scene_text": {"type": "string", "description": "Full .tscn/.tres source text."},
        "scene_file": {"type": "string", "description": "Path to a .tscn. res:// requires root."},
        "root": {"type": "string", "description": "Absolute project directory; resolves res:// and confines reads."}
    });
    let with = |extra: Value| {
        let mut p = scene_props.clone();
        if let (Some(base), Some(add)) = (p.as_object_mut(), extra.as_object()) {
            for (k, v) in add {
                base.insert(k.clone(), v.clone());
            }
        }
        p
    };
    json!([
        {
            "name": "scene_lint",
            "title": "Lint a Godot scene",
            "description": format!("Check a Godot 4 scene for problems the editor only shows at runtime: physics bodies without collision shapes, signal connections to missing nodes or methods. Use before exporting or after editing a .tscn. {SCENE_SOURCE} Returns count and findings[{{code, message, node}}]; an empty list means clean."),
            "inputSchema": {"type": "object", "properties": scene_props},
            "outputSchema": envelope(&json!({"count": {"type": "integer"}, "findings": {"type": "array", "items": {"type": "object"}}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "spatial_query",
            "title": "Bounding box of a node",
            "description": format!("World-space axis-aligned bounding box of one node and everything under it, after all parent transforms. Use to check sizes, overlaps, or whether something sits where you expect. {SCENE_SOURCE} Returns node, empty, min[x,y,z], max[x,y,z], size[x,y,z]."),
            "inputSchema": {"type": "object", "properties": with(json!({"node_path": {"type": "string", "description": "Scene path from the root, e.g. 'Level/Platform3', or '.' for the whole scene."}})), "required": ["node_path"]},
            "outputSchema": envelope(&json!({"node": {"type": "string"}, "empty": {"type": "boolean"}, "min": {"type": "array"}, "max": {"type": "array"}, "size": {"type": "array"}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "scene_inspect",
            "title": "Inspect a Godot scene",
            "description": format!("Read-only views of a parsed scene. operation='tree': paginated node list (pass next_offset back as offset for the next page); 'signals': every connection with resolved/reason; 'property': one node's transform and properties (needs node_path); 'find': substring search over node names and types (needs query). {SCENE_SOURCE}"),
            "inputSchema": {"type": "object", "properties": with(json!({
                "operation": {"type": "string", "enum": ["tree", "signals", "property", "find"], "default": "tree"},
                "node_path": {"type": "string", "description": "Required for operation='property'."},
                "query": {"type": "string", "description": "Required for operation='find'."},
                "depth": {"type": "integer", "default": 64, "description": "Max tree depth for 'tree'."},
                "limit": {"type": "integer", "default": 100, "description": "Page size for 'tree'."},
                "offset": {"type": "integer", "default": 0, "description": "Page start; use next_offset from the previous page."}
            }))},
            "outputSchema": envelope(&json!({"operation": {"type": "string"}, "total": {"type": "integer"}, "next_offset": {"type": ["integer", "null"]}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "export_scene",
            "title": "Export a scene spec",
            "description": "Turn a JSON scene spec into engine text, deterministically (same spec, same bytes, same uid). format='godot4' gives Godot 4 .tscn, format='gltf' gives glTF 2.0 (primitive meshes, PBR colours, lights, camera; collision shapes and scripts are left out); further engines will be format values on this tool, never new tools. Returns format, uid, nodes, bytes and the text itself; nothing is written to disk.",
            "inputSchema": {"type": "object", "properties": {
                "format": {"type": "string", "enum": ["godot4", "gltf"], "default": "godot4", "description": "godot4 = .tscn text; gltf = glTF 2.0 JSON with the buffer embedded (loads in Blender, three.js, Bevy, Unity, Resonite)."},
                "scene": {
                    "type": "object",
                    "description": "Scene spec: {uid?, format?, nodes[], connections[], ext_resources[], sub_resources[]}. Node: {name, type, parent?, unique?, position?, rotation_degrees?, scale?, properties?}. A property value is a JSON scalar, a 3-number array (Vector3), or {\"raw\": \"<godot syntax>\"}.",
                    "required": ["nodes"],
                    "properties": {
                        "uid": {"type": "string"}, "format": {"type": "integer"},
                        "nodes": {"type": "array", "items": {"type": "object"}},
                        "connections": {"type": "array", "items": {"type": "object"}},
                        "ext_resources": {"type": "array", "items": {"type": "object"}},
                        "sub_resources": {"type": "array", "items": {"type": "object"}}
                    }
                }
            }, "required": ["scene"]},
            "outputSchema": envelope(&json!({"format": {"type": "string"}, "uid": {"type": "string"}, "nodes": {"type": "integer"}, "bytes": {"type": "integer"}, "text": {"type": "string"}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_list",
            "title": "List playable games",
            "description": "Every game in the repo: forge-made Godot projects (depot/games) and three.js pages (web/games), with group, one-line description and the last gate result (pass, what failed, when). Use first to find a slug. Optional filter matches title, slug, group or kind ('web', 'godot'). Returns count and games[{kind, slug, title, group, description, verify}].",
            "inputSchema": {"type": "object", "properties": {
                "filter": {"type": "string", "description": "Free text, e.g. 'calm', 'benny', 'godot'. Empty lists all."},
                "kind": {"type": "string", "enum": ["godot", "web"], "description": "Only one runtime."},
                "root": {"type": "string", "description": "Repo root; found automatically from GAMECRAFT_ROOT or the working directory when omitted."}
            }},
            "outputSchema": envelope(&json!({"count": {"type": "integer"}, "games": {"type": "array", "items": {"type": "object"}}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_report",
            "title": "One game in detail",
            "description": "Everything known about one game: catalog entry, the player cheat sheet, the README, the full last gate record (web: verify.json with checkpoints and self-test; Godot: trace and lint) and its screenshot names. Use to explain a game or diagnose a failing gate. Returns game, cheatsheet, readme, gate, shots, thumb.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list, e.g. 'where-is-benny'."},
                "kind": {"type": "string", "enum": ["godot", "web"], "description": "Only needed if both runtimes have this slug."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"game": {"type": "object"}, "cheatsheet": {"type": ["string", "null"]}, "gate": {}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_verify",
            "title": "Run a game's gate",
            "description": "OPT-IN: run one game's verification gate now and return the fresh result. Web games run the headless-Chromium self-play gate (needs GAMECRAFT_MCP_ALLOW_WEB=1, node and bun; rewrites verify.json and shots/). Godot games boot headless Godot and look for a win marker (needs GAMECRAFT_MCP_ALLOW_GODOT=1). Takes 10 s to a few minutes. Returns pass plus the gate record.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list."},
                "kind": {"type": "string", "enum": ["godot", "web"]},
                "timeout_s": {"type": "integer", "default": 300, "description": "Wall-clock limit; the run is killed after it."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"pass": {"type": "boolean"}, "kind": {"type": "string"}})),
            "annotations": annotations(false, false),
        },
        {
            "name": "game_create",
            "title": "Create a Godot game from a spec",
            "description": "Compile a GameSpec JSON into a new, verified Godot project under depot/games/<slug>/: the level is checked for completability by a Rust replay and the scene is linted before anything is written. You (the model) design the spec; no other model is called. Archetypes: platformer, runner, pong, maze, flight, driving, chess, talk. Call with explain=true first to get the full spec format. Overwrites an existing project with the same slug.",
            "inputSchema": {"type": "object", "properties": {
                "spec": {"type": "object", "description": "GameSpec: {slug, title, archetype, theme, physics, level, maze?, flight?, drive?, chess?, talk?}. Missing fields take defaults."},
                "explain": {"type": "boolean", "default": false, "description": "Return the spec format and design rules instead of creating anything."},
                "root": {"type": "string", "description": "Repo root (the game lands in <root>/depot/games/<slug>); found automatically when omitted."}
            }},
            "outputSchema": envelope(&json!({"slug": {"type": "string"}, "dir": {"type": "string"}, "summary": {"type": "string"}, "guide": {"type": "string"}})),
            "annotations": annotations(false, true),
        },
        {
            "name": "scene_verify",
            "title": "Boot a Godot project headless",
            "description": "OPT-IN (GAMECRAFT_MCP_ALLOW_GODOT=1): boot any Godot project directory headless and require a win marker. Prefer game_verify for games in the catalog; use this for a project elsewhere. The engine is killed on timeout and when this server exits. Returns won, markers, elapsed_ms, timed_out, engine_errors and the tail of engine output.",
            "inputSchema": {"type": "object", "properties": {
                "game_path": {"type": "string", "description": "Project directory containing project.godot and main.tscn."},
                "frames": {"type": "integer", "default": 3600, "description": "Frame budget passed to --quit-after (60 fps)."},
                "autoplay": {"type": "boolean", "default": true, "description": "Let the built-in driver play (GAMECRAFT_AUTOPLAY=1)."},
                "timeout_s": {"type": "integer", "description": "Wall-clock limit (default frames/60*2+60)."}
            }, "required": ["game_path"]},
            "outputSchema": envelope(&json!({"won": {"type": "boolean"}, "markers": {"type": "array"}, "timed_out": {"type": "boolean"}})),
            "annotations": annotations(false, false),
        },
    ])
}

// ------------------------------------------------------------------ results

/// A tool failure the model can act on.
struct ToolError {
    code: &'static str,
    message: String,
    suggestion: String,
    remediation: String,
}

fn fail(
    code: &'static str,
    message: impl Into<String>,
    suggestion: impl Into<String>,
    remediation: impl Into<String>,
) -> ToolError {
    ToolError {
        code,
        message: message.into(),
        suggestion: suggestion.into(),
        remediation: remediation.into(),
    }
}

fn ok_result(summary: &str, data: Value) -> Value {
    let mut structured = if data.is_object() {
        data
    } else {
        json!({ "data": data })
    };
    if let Some(map) = structured.as_object_mut() {
        map.insert("success".to_string(), Value::Bool(true));
        map.insert("message".to_string(), Value::String(summary.to_string()));
    }
    json!({
        "content": [
            {"type": "text", "text": summary},
            {"type": "text", "text": structured.to_string()}
        ],
        "structuredContent": structured,
        "isError": false,
    })
}

fn error_result(err: &ToolError) -> Value {
    let structured = json!({
        "success": false,
        "error": err.code,
        "message": err.message,
        "dialogic": {"suggestion": err.suggestion, "remediation": err.remediation},
    });
    json!({
        "content": [{"type": "text", "text": format!("{}\nWhat to do: {}\nNext: {}", err.message, err.suggestion, err.remediation)}],
        "structuredContent": structured,
        "isError": true,
    })
}

/// Run one tool. Unknown tool = protocol error; everything else is a tool result.
fn call_tool(name: &str, args: &Value) -> Result<Value, (i32, String)> {
    let outcome = match name {
        "scene_lint" => tool_scene_lint(args),
        "spatial_query" => tool_spatial_query(args),
        "scene_inspect" => tool_scene_inspect(args),
        "export_scene" => tool_export_scene(args),
        "game_list" => tool_game_list(args),
        "game_report" => tool_game_report(args),
        "game_verify" => tool_game_verify(args),
        "game_create" => tool_game_create(args),
        "scene_verify" => tool_scene_verify(args),
        _ => {
            return Err((
                -32602,
                format!("unknown tool: {name}. Call tools/list for the available tools."),
            ))
        }
    };
    Ok(match outcome {
        Ok((summary, data)) => ok_result(summary.as_str(), data),
        Err(err) => error_result(&err),
    })
}

type ToolOutcome = Result<(String, Value), ToolError>;

fn scene_source(args: &Value) -> Result<String, ToolError> {
    let text = args.get("scene_text").and_then(Value::as_str);
    let file = args.get("scene_file").and_then(Value::as_str);
    let root = args.get("root").and_then(Value::as_str);
    if text.is_none() && file.is_none() {
        return Err(fail(
            "missing_scene",
            "No scene given.",
            "Pass the .tscn text as scene_text, or a path as scene_file.",
            "scene_lint {\"scene_file\": \"D:/project/main.tscn\"}",
        ));
    }
    gamecraft_mcp::resolve_scene_text(text, file, root).map_err(|reason| {
        fail(
            "scene_unreadable",
            format!("Could not read the scene: {reason}"),
            "Check the path; res:// paths need root set to the project folder.",
            "game_list to find a project, then game_report for its folder",
        )
    })
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn tool_scene_lint(args: &Value) -> ToolOutcome {
    let text = scene_source(args)?;
    let findings = scene_lint(text.as_str());
    let summary = match findings.first() {
        None => "No findings: the scene is clean.".to_string(),
        Some(first) => format!(
            "{}; first: {} at {} ({}).",
            plural(findings.len(), "finding", "findings"),
            first.code,
            first.node,
            first.message
        ),
    };
    let data = json!({"count": findings.len(), "findings": findings});
    Ok((summary, data))
}

fn tool_spatial_query(args: &Value) -> ToolOutcome {
    let text = scene_source(args)?;
    let Some(path) = args.get("node_path").and_then(Value::as_str) else {
        return Err(fail(
            "missing_node_path",
            "node_path is required.",
            "Use '.' for the whole scene or a path like 'Level/Platform3'.",
            "scene_inspect {\"operation\": \"tree\"} to see the node paths",
        ));
    };
    let mut value = spatial_query(text.as_str(), path).map_err(|reason| {
        fail(
            "node_not_found",
            format!("Cannot measure '{path}': {reason}"),
            "Node paths are relative to the scene root, separated by '/'.",
            format!(
                "scene_inspect {{\"operation\": \"find\", \"query\": \"{}\"}}",
                path.rsplit('/').next().unwrap_or(path)
            ),
        )
    })?;
    let triple = |key: &str| -> Vec<f64> {
        value
            .get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default()
    };
    let (min, max) = (triple("min"), triple("max"));
    let size: Vec<f64> = min
        .iter()
        .zip(max.iter())
        .map(|(a, b)| ((b - a) * 1000.0).round() / 1000.0)
        .collect();
    let empty = value.get("empty").and_then(Value::as_bool).unwrap_or(false);
    let summary = if empty || size.len() != 3 {
        format!("'{path}' has no geometry, so its box is empty.")
    } else {
        format!(
            "'{path}' spans {:.2} x {:.2} x {:.2} m, from {:?} to {:?}.",
            size[0],
            size[1],
            size[2],
            round3(&min),
            round3(&max)
        )
    };
    if let Some(map) = value.as_object_mut() {
        map.insert("size".to_string(), json!(size));
    }
    Ok((summary, value))
}
fn round3(v: &[f64]) -> Vec<f64> {
    v.iter().map(|x| (x * 1000.0).round() / 1000.0).collect()
}

fn usize_arg(args: &Value, key: &str, fallback: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .map_or(fallback, |value| usize::try_from(value).unwrap_or(fallback))
}

fn tool_scene_inspect(args: &Value) -> ToolOutcome {
    let text = scene_source(args)?;
    let operation = args
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or(gamecraft_mcp::OP_TREE);
    let options = gamecraft_mcp::InspectOptions {
        operation,
        node_path: args.get("node_path").and_then(Value::as_str),
        query: args.get("query").and_then(Value::as_str),
        depth: usize_arg(args, "depth", gamecraft_mcp::DEFAULT_DEPTH),
        limit: usize_arg(args, "limit", gamecraft_mcp::DEFAULT_LIMIT),
        offset: usize_arg(args, "offset", 0),
    };
    let value = gamecraft_mcp::scene_inspect(text.as_str(), &options).map_err(|reason| {
        fail(
            "inspect_failed",
            format!("scene_inspect {operation} failed: {reason}"),
            "operation 'property' needs node_path, 'find' needs query.",
            "scene_inspect {\"operation\": \"tree\"}",
        )
    })?;
    let total = value.get("total").and_then(Value::as_u64).unwrap_or(0);
    let summary = match operation {
        "signals" => format!(
            "{} connections, {} unresolved.",
            total,
            value.get("unresolved").and_then(Value::as_u64).unwrap_or(0)
        ),
        "property" => format!(
            "Properties of {}.",
            value
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("the node")
        ),
        "find" => format!("{} nodes match '{}'.", total, options.query.unwrap_or("")),
        _ => match value.get("next_offset").and_then(Value::as_u64) {
            Some(next) => {
                format!("{total} nodes; this page ends before {next}, pass offset={next} for more.")
            }
            None => format!("{total} nodes, all on this page."),
        },
    };
    Ok((summary, value))
}

fn tool_export_scene(args: &Value) -> ToolOutcome {
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or(gamecraft_mcp::spec::FORMAT_GODOT4);
    if !gamecraft_mcp::spec::FORMATS.contains(&format) {
        return Err(fail(
            "unsupported_format",
            format!("There is no '{format}' export."),
            format!(
                "Use one of: {}. glTF loads in Blender, Unity, three.js and most other engines.",
                gamecraft_mcp::spec::FORMATS.join(", ")
            ),
            "export_scene {\"format\": \"gltf\", \"scene\": <same scene>}",
        ));
    }
    let Some(scene) = args.get("scene") else {
        return Err(fail(
            "missing_scene",
            "scene is required.",
            "Pass a scene spec object with at least nodes[].",
            "export_scene {\"scene\": {\"nodes\": [{\"name\": \"World\", \"type\": \"Node3D\"}]}}",
        ));
    };
    let spec: gamecraft_mcp::SceneSpec = serde_json::from_value(scene.clone()).map_err(|err| {
        fail(
            "invalid_spec",
            format!("The scene spec does not parse: {err}"),
            "Check field names and types against the scene description in tools/list.",
            "export_scene with a minimal spec, then add nodes",
        )
    })?;
    let outcome = gamecraft_mcp::export_spec(&spec, format).map_err(|reason| {
        fail(
            "export_failed",
            format!("Export failed: {reason}"),
            "Every parent must name an earlier node; resources must be declared before use.",
            "scene_lint on the exported text once it exports",
        )
    })?;
    let summary = format!(
        "Exported {} as {format}: {} bytes, uid {}.",
        plural(outcome.nodes, "node", "nodes"),
        outcome.text.len(),
        outcome.uid
    );
    Ok((
        summary,
        json!({"format": format, "uid": outcome.uid, "nodes": outcome.nodes, "bytes": outcome.text.len(), "text": outcome.text}),
    ))
}

fn repo_root(args: &Value) -> Result<std::path::PathBuf, ToolError> {
    catalog::find_root(args.get("root").and_then(Value::as_str)).ok_or_else(|| {
        fail("repo_not_found", "Could not find the GameCraft repo (a folder with depot/games or web/games).", "Pass root, or start the server with GAMECRAFT_ROOT set, or run it from inside the repo.", "game_list {\"root\": \"D:/Dev/repos/gamecraft\"}")
    })
}
fn kind_arg(args: &Value) -> Option<GameKind> {
    match args.get("kind").and_then(Value::as_str) {
        Some("godot") => Some(GameKind::Godot),
        Some("web") => Some(GameKind::Web),
        _ => None,
    }
}
fn find_game(args: &Value) -> Result<(std::path::PathBuf, catalog::GameEntry), ToolError> {
    let root = repo_root(args)?;
    let Some(slug) = args.get("slug").and_then(Value::as_str) else {
        return Err(fail(
            "missing_slug",
            "slug is required.",
            "Use a folder name from game_list.",
            "game_list",
        ));
    };
    let games = catalog::discover(root.as_path());
    if let Some(game) = catalog::find(&games, slug, kind_arg(args)) {
        return Ok((root, game.clone()));
    }
    let close = catalog::suggest(&games, slug);
    Err(fail(
        "unknown_game",
        format!("There is no game '{slug}'."),
        if close.is_empty() {
            "List the games and use one of their slugs.".to_string()
        } else {
            format!("Did you mean: {}?", close.join(", "))
        },
        close.first().map_or_else(
            || "game_list".to_string(),
            |s| format!("game_report {{\"slug\": \"{s}\"}}"),
        ),
    ))
}

fn tool_game_list(args: &Value) -> ToolOutcome {
    let root = repo_root(args)?;
    let filter = args.get("filter").and_then(Value::as_str).unwrap_or("");
    let kind = kind_arg(args);
    let games: Vec<catalog::GameEntry> = catalog::discover(root.as_path())
        .into_iter()
        .filter(|g| kind.is_none_or(|k| g.kind == k) && catalog::matches(g, filter))
        .collect();
    let failing: Vec<&str> = games
        .iter()
        .filter(|g| g.verify.as_ref().is_some_and(|v| !v.pass))
        .map(|g| g.slug.as_str())
        .collect();
    let unverified = games.iter().filter(|g| g.verify.is_none()).count();
    let godot = games.iter().filter(|g| g.kind == GameKind::Godot).count();
    let mut summary = format!(
        "{} ({} Godot, {} web)",
        plural(games.len(), "game", "games"),
        godot,
        games.len() - godot
    );
    if failing.is_empty() {
        summary.push_str(if unverified == 0 {
            "; all passed their last gate."
        } else {
            "; none failing."
        });
    } else {
        let _ = write!(summary, "; failing last gate: {}.", failing.join(", "));
    }
    if unverified > 0 {
        let _ = write!(
            summary,
            " {} never verified.",
            plural(unverified, "game", "games")
        );
    }
    if games.is_empty() && !filter.is_empty() {
        return Err(fail(
            "no_match",
            format!("No game matches '{filter}'."),
            "Try a shorter filter, or none.",
            "game_list",
        ));
    }
    Ok((summary, json!({"count": games.len(), "games": games})))
}

fn tool_game_report(args: &Value) -> ToolOutcome {
    let (_, game) = find_game(args)?;
    let status = match &game.verify {
        Some(v) if v.pass => format!("passed its last gate ({})", v.progress),
        Some(v) => format!(
            "FAILED its last gate: {}",
            v.failed.clone().unwrap_or_default()
        ),
        None => "has never been verified".to_string(),
    };
    let summary = format!(
        "{} ({} game, {}) {status}.",
        game.title,
        game.kind.as_str(),
        game.group
    );
    Ok((summary, catalog::report(&game)))
}

fn tool_game_verify(args: &Value) -> ToolOutcome {
    let (root, game) = find_game(args)?;
    let timeout = args.get("timeout_s").and_then(Value::as_u64).unwrap_or(300);
    match game.kind {
        GameKind::Web => {
            if std::env::var("GAMECRAFT_MCP_ALLOW_WEB").ok().as_deref() != Some("1") {
                return Err(fail("web_gate_disabled", "Running the web gate is turned off on this server.", "It launches node, bun and headless Chromium, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_WEB=1, or read the last result instead.", format!("game_report {{\"slug\": \"{}\"}}", game.slug)));
            }
            let run = catalog::web_verify(root.as_path(), game.slug.as_str(), timeout)
                .map_err(|reason| fail("web_gate_failed_to_run", reason, "Check that node and bun are installed and web/node_modules exists (bun install in web/).", "game_report for the previous result"))?;
            let pass = run
                .pointer("/verify/pass")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let failed = run
                .pointer("/verify/failed")
                .and_then(Value::as_str)
                .unwrap_or("");
            let summary = if pass {
                format!("{} passes its gate.", game.title)
            } else {
                format!("{} FAILS its gate: {failed}.", game.title)
            };
            let mut data = run;
            if let Some(map) = data.as_object_mut() {
                map.insert("pass".to_string(), Value::Bool(pass));
                map.insert("kind".to_string(), json!("web"));
            }
            Ok((summary, data))
        }
        GameKind::Godot => {
            if std::env::var("GAMECRAFT_MCP_ALLOW_GODOT").ok().as_deref() != Some("1") {
                return Err(fail("godot_gate_disabled", "Running Godot is turned off on this server.", "It launches a local Godot process, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_GODOT=1, or read the last recorded trace instead.", format!("game_report {{\"slug\": \"{}\", \"kind\": \"godot\"}}", game.slug)));
            }
            let run = catalog::godot_verify(game.dir.as_path(), 3600, true, Some(timeout))
                .map_err(|reason| {
                    fail(
                        "godot_failed_to_run",
                        reason,
                        "Set GAMECRAFT_GODOT to the Godot 4 executable if it is not on PATH.",
                        "game_report for the recorded trace",
                    )
                })?;
            let won = run.get("won").and_then(Value::as_bool).unwrap_or(false);
            let summary = if won {
                format!("{} reached its win marker in headless Godot.", game.title)
            } else {
                format!("{} did NOT reach a win marker.", game.title)
            };
            let mut data = run;
            if let Some(map) = data.as_object_mut() {
                map.insert("pass".to_string(), Value::Bool(won));
                map.insert("kind".to_string(), json!("godot"));
            }
            Ok((summary, data))
        }
    }
}

fn tool_game_create(args: &Value) -> ToolOutcome {
    if args
        .get("explain")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || args.get("spec").is_none()
    {
        let guide = gamecraft_forge::llm::spec_prompt("<your idea>");
        if args.get("spec").is_none()
            && !args
                .get("explain")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            return Err(fail(
                "missing_spec",
                "No spec given.",
                "Read the spec format first, then call again with spec.",
                "game_create {\"explain\": true}",
            ));
        }
        return Ok(("The GameSpec format and design rules are in guide; design a spec and call game_create with it.".to_string(), json!({"guide": guide})));
    }
    let root = repo_root(args)?;
    let spec: gamecraft_forge::create::GameSpec = serde_json::from_value(
        args.get("spec").cloned().unwrap_or(Value::Null),
    )
    .map_err(|err| {
        fail(
            "invalid_spec",
            format!("The spec does not parse: {err}"),
            "Field names and types must match the GameSpec format.",
            "game_create {\"explain\": true}",
        )
    })?;
    if let Err(reason) = gamecraft_forge::create::validate(&spec) {
        return Err(fail(
            "spec_rejected",
            format!("The spec was rejected: {reason}"),
            "Fix every listed problem; the rules are in the guide.",
            "game_create {\"explain\": true}",
        ));
    }
    let summary = gamecraft_forge::create::create_game(&spec, root.as_path())
        .map_err(|reason| fail("not_completable", format!("The game was not written: {reason}"), "Usually a gap too wide to jump or a platform too low; the Rust replay must reach the goal.", "game_create {\"explain\": true} for the physics limits"))?;
    let dir = root.join("depot").join("games").join(spec.slug.as_str());
    Ok((
        format!(
            "Created '{}' ({}) in {}: {summary}",
            spec.title,
            spec.archetype,
            dir.display()
        ),
        json!({"slug": spec.slug, "dir": dir, "summary": summary}),
    ))
}

fn tool_scene_verify(args: &Value) -> ToolOutcome {
    if std::env::var("GAMECRAFT_MCP_ALLOW_GODOT").ok().as_deref() != Some("1") {
        return Err(fail("godot_gate_disabled", "scene_verify is turned off on this server.", "It launches a local Godot process, so it is opt-in. Restart the server with GAMECRAFT_MCP_ALLOW_GODOT=1.", "scene_lint on the project's main.tscn needs no engine"));
    }
    let Some(game) = args.get("game_path").and_then(Value::as_str) else {
        return Err(fail(
            "missing_game_path",
            "game_path is required.",
            "Pass the folder holding project.godot.",
            "game_list to find catalog games",
        ));
    };
    let frames = args.get("frames").and_then(Value::as_u64).unwrap_or(3600);
    let autoplay = args
        .get("autoplay")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let run = catalog::godot_verify(
        std::path::Path::new(game),
        frames,
        autoplay,
        args.get("timeout_s").and_then(Value::as_u64),
    )
    .map_err(|reason| {
        fail(
            "godot_failed_to_run",
            reason,
            "Set GAMECRAFT_GODOT to the Godot 4 executable if it is not on PATH.",
            "scene_lint on main.tscn",
        )
    })?;
    let won = run.get("won").and_then(Value::as_bool).unwrap_or(false);
    let timed_out = run
        .get("timed_out")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let summary = if won {
        "Win marker reached.".to_string()
    } else if timed_out {
        "No win: the engine hit the time limit.".to_string()
    } else {
        "No win marker within the frame budget.".to_string()
    };
    Ok((summary, run))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, args: &Value) -> Value {
        call_tool(name, args).unwrap_or_else(|(c, m)| json!({"rpc_error": c, "message": m}))
    }

    /// A modern (2026-07-28) request.
    fn modern(method: &str, version: &str) -> Value {
        json!({"jsonrpc": "2.0", "id": 7, "method": method, "params": {"_meta": {
            META_VERSION: version,
            "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"},
            "io.modelcontextprotocol/clientCapabilities": {}
        }}})
    }

    #[test]
    fn legacy_initialize_negotiates_known_versions_and_falls_back_to_newest_legacy() {
        assert_eq!(
            negotiate(&json!({"params": {"protocolVersion": "2024-11-05"}})),
            "2024-11-05"
        );
        assert_eq!(
            negotiate(&json!({"params": {"protocolVersion": "2025-11-25"}})),
            "2025-11-25"
        );
        assert_eq!(
            negotiate(&json!({"params": {"protocolVersion": "2099-01-01"}})),
            "2025-11-25"
        );
    }

    #[test]
    fn modern_discover_advertises_both_eras() {
        let msg = modern("server/discover", "2026-07-28");
        let reply = respond("server/discover", &msg, &json!(7));
        assert_eq!(
            reply.pointer("/result/resultType"),
            Some(&json!("complete"))
        );
        assert_eq!(
            reply.pointer("/result/supportedVersions/0"),
            Some(&json!("2026-07-28"))
        );
        assert_eq!(
            reply.pointer("/result/supportedVersions/1"),
            Some(&json!("2025-11-25"))
        );
        let name = reply
            .pointer("/result/_meta")
            .and_then(|m| m.get(META_SERVER_INFO))
            .and_then(|i| i.get("name"));
        assert_eq!(name, Some(&json!("gamecraft-mcp")));
    }

    #[test]
    fn modern_tools_list_is_cacheable_and_stamped() {
        let msg = modern("tools/list", "2026-07-28");
        let reply = respond("tools/list", &msg, &json!(7));
        assert_eq!(reply.pointer("/result/ttlMs"), Some(&json!(TTL_MS)));
        assert_eq!(reply.pointer("/result/cacheScope"), Some(&json!("public")));
        assert_eq!(
            reply.pointer("/result/resultType"),
            Some(&json!("complete"))
        );
        // Legacy clients get the plain shape.
        let legacy = respond("tools/list", &json!({"method": "tools/list"}), &json!(8));
        assert!(legacy.pointer("/result/resultType").is_none());
        assert!(legacy.pointer("/result/ttlMs").is_none());
    }

    #[test]
    fn unknown_modern_version_lists_what_is_supported() {
        let msg = modern("tools/list", "1900-01-01");
        let reply = respond("tools/list", &msg, &json!(7));
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32_022)));
        assert_eq!(
            reply.pointer("/error/data/requested"),
            Some(&json!("1900-01-01"))
        );
        assert_eq!(
            reply.pointer("/error/data/supported/0"),
            Some(&json!("2026-07-28"))
        );
    }

    #[test]
    fn modern_tool_call_result_is_stamped() {
        let mut msg = modern("tools/call", "2026-07-28");
        if let Some(params) = msg.get_mut("params").and_then(Value::as_object_mut) {
            params.insert("name".to_string(), json!("scene_lint"));
            params.insert(
                "arguments".to_string(),
                json!({"scene_text": "[gd_scene format=3]\n\n[node name=\"R\" type=\"Node3D\"]\n"}),
            );
        }
        let reply = respond("tools/call", &msg, &json!(7));
        assert_eq!(
            reply.pointer("/result/resultType"),
            Some(&json!("complete"))
        );
        assert_eq!(reply.pointer("/result/isError"), Some(&json!(false)));
    }

    #[test]
    fn every_tool_has_title_annotations_and_output_schema() {
        let tools = tool_list();
        let list = tools.as_array().map(Vec::as_slice).unwrap_or_default();
        assert_eq!(list.len(), 9);
        for t in list {
            assert!(
                t.get("title").is_some()
                    && t.get("annotations").is_some()
                    && t.get("outputSchema").is_some()
            );
            let d = t.get("description").and_then(Value::as_str).unwrap_or("");
            assert!(d.len() > 80, "description too thin: {d}");
        }
    }

    #[test]
    fn tool_failures_are_results_with_remediation_not_rpc_errors() {
        let r = call("scene_lint", &json!({}));
        assert_eq!(r.get("isError"), Some(&Value::Bool(true)));
        assert!(r
            .pointer("/structuredContent/dialogic/remediation")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()));
        let unknown = call("no_such_tool", &json!({}));
        assert!(unknown.get("rpc_error").is_some());
    }

    #[test]
    fn success_carries_summary_text_and_structured_data() {
        let scene = "[gd_scene format=3]\n\n[node name=\"World\" type=\"Node3D\"]\n";
        let r = call("scene_lint", &json!({"scene_text": scene}));
        assert_eq!(r.get("isError"), Some(&Value::Bool(false)));
        assert!(r
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .is_some_and(|s| s.starts_with("No findings")));
        assert_eq!(
            r.pointer("/structuredContent/success"),
            Some(&Value::Bool(true))
        );
        assert_eq!(r.pointer("/structuredContent/count"), Some(&json!(0)));
    }

    #[test]
    fn opt_in_gates_explain_how_to_enable() {
        if std::env::var("GAMECRAFT_MCP_ALLOW_GODOT").is_ok() {
            return;
        }
        let r = call("scene_verify", &json!({"game_path": "."}));
        assert_eq!(
            r.pointer("/structuredContent/error"),
            Some(&json!("godot_gate_disabled"))
        );
        assert!(r
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .is_some_and(|s| s.contains("GAMECRAFT_MCP_ALLOW_GODOT=1")));
    }

    #[test]
    fn game_create_without_spec_points_to_the_guide() {
        let r = call("game_create", &json!({}));
        assert!(r
            .pointer("/structuredContent/dialogic/remediation")
            .and_then(Value::as_str)
            .is_some_and(|s| s.contains("explain")));
        let g = call("game_create", &json!({"explain": true}));
        assert!(g
            .pointer("/structuredContent/guide")
            .and_then(Value::as_str)
            .is_some_and(|s| s.contains("archetype")));
    }
}
