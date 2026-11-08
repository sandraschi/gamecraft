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

use gamecraft_mcp::catalog;
use gamecraft_mcp::launch::Mode;
use gamecraft_mcp::registry;
use gamecraft_mcp::tools::{
    fail, tool_export_scene, tool_game_create, tool_game_describe, tool_game_doctor,
    tool_game_help, tool_game_list, tool_game_report, tool_game_running, tool_game_start,
    tool_game_stop, tool_game_verify, tool_scene_inspect, tool_scene_lint, tool_scene_verify,
    tool_spatial_query, ToolError, ToolOutcome,
};
use serde_json::{json, Value};
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
Start with game_list to see what exists and how each game last verified; game_describe says what one game is, its goal and controls and how to start it; game_help gives the overview or a full cheat sheet; game_report adds the gate record. \
To show someone a game: game_play opens it (web games in the browser, others in their own window), game_demo lets it play itself, game_running and game_stop manage what was started; starting and stopping need GAMECRAFT_MCP_ALLOW_LAUNCH=1. Cheat sheets are also resources. \
scene_lint / scene_inspect / spatial_query read .tscn text; export_scene and game_create write games from JSON specs. \
Running engines or browsers is opt-in: scene_verify needs GAMECRAFT_MCP_ALLOW_GODOT=1, game_verify on web games needs GAMECRAFT_MCP_ALLOW_WEB=1, on Bevy games GAMECRAFT_MCP_ALLOW_BEVY=1. \
Every result starts with a one-line summary; failures carry a suggestion and a remediation.";

fn main() {
    // One binary, CLI first (Craft contract): `commands`, `describe` and `run` use the command
    // registry; no arguments or `mcp` speaks MCP over stdio, as every existing client config does.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a != "mcp") {
        let Some(out) = registry::cli_args(&args) else {
            eprintln!("unknown subcommand '{}'\n{}", args[0], registry::CLI_USAGE);
            std::process::exit(2);
        };
        if !out.stdout.is_empty() {
            println!("{}", out.stdout.trim_end());
        }
        if !out.stderr.is_empty() {
            eprintln!("{}", out.stderr.trim_end());
        }
        std::process::exit(out.code);
    }
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
            "capabilities": {"tools": {"listChanged": false}, "resources": {"listChanged": false}},
            "serverInfo": server_info(),
            "instructions": INSTRUCTIONS,
        })),
        "server/discover" => Ok(json!({
            "supportedVersions": all_versions(),
            "capabilities": {"tools": {"listChanged": false}, "resources": {"listChanged": false}},
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
        "resources/list" => Ok(resource_list()),
        "resources/read" => {
            let uri = msg
                .pointer("/params/uri")
                .and_then(Value::as_str)
                .ok_or_else(|| (-32602_i32, "missing uri".to_string()))?;
            resource_read(uri)
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
    let mut tools = json!([
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
            "description": "Every game in the repo: forge-made Godot projects (depot/games), three.js pages (web/games) and native Bevy games (bevy/games), with group, one-line description and the last gate result (pass, what failed, when). Use first to find a slug. Optional filter matches title, slug, group or kind ('web', 'godot', 'bevy'). Returns count and games[{kind, slug, title, group, description, verify}].",
            "inputSchema": {"type": "object", "properties": {
                "filter": {"type": "string", "description": "Free text, e.g. 'calm', 'benny', 'godot'. Empty lists all."},
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"], "description": "Only one runtime."},
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
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"], "description": "Only needed if both runtimes have this slug."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"game": {"type": "object"}, "cheatsheet": {"type": ["string", "null"]}, "gate": {}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_describe",
            "title": "What a game is and how to start it",
            "description": "One game in plain terms, generated from its files: title, runtime, group, description, whether it is playable at all (some entries are data or plans), goal and controls from its cheat sheet, the last gate result and the exact commands to start it. Use before game_play to tell the user what they are about to see. Returns slug, title, kind, group, description, playable, not_playable_reason, goal[], controls[], gate, run{}.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list."},
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"], "description": "Only needed if both runtimes have this slug."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"slug": {"type": "string"}, "playable": {"type": "boolean"}, "goal": {"type": "array"}, "controls": {"type": "array"}, "run": {"type": "object"}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_help",
            "title": "GameCraft help and cheat sheets",
            "description": "Help text generated from the repo. No topic: the overview of every command. topic = overview, games (every game with a one-line description), start, play, demo, running, stop, doctor, mcp, or a game slug (description plus the full cheat sheet). Returns topic and text.",
            "inputSchema": {"type": "object", "properties": {
                "topic": {"type": "string", "description": "A help topic or a game slug. Empty gives the overview."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }},
            "outputSchema": envelope(&json!({"topic": {"type": ["string", "null"]}, "text": {"type": "string"}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_start",
            "title": "Start a game",
            "description": "OPT-IN (GAMECRAFT_MCP_ALLOW_LAUNCH=1). Bring a game up. Web games: starts the local arcade server if needed and returns the URL, without opening a browser unless open_browser is true. Godot and Bevy games: opens the game in its own window (a Bevy game must be built first; the error gives the cargo command). mode 'demo' lets the game play itself. A game started here ends when this server exits. Prefer game_play or game_demo to actually show someone a game.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list."},
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"]},
                "mode": {"type": "string", "enum": ["play", "demo"], "default": "play"},
                "open_browser": {"type": "boolean", "default": false, "description": "Web games only: also open the URL in the default browser."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"slug": {"type": "string"}, "kind": {"type": "string"}, "mode": {"type": "string"}, "pid": {"type": ["integer", "null"]}, "url": {"type": ["string", "null"]}, "opened": {"type": "boolean"}})),
            "annotations": annotations(false, false),
        },
        {
            "name": "game_play",
            "title": "Play a game",
            "description": "OPT-IN (GAMECRAFT_MCP_ALLOW_LAUNCH=1). Open a game for the user to play: web games in their default browser, Godot and Bevy games in their own window. Starts the arcade server first when needed. Same results as game_start with the browser opened.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list."},
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"]},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"slug": {"type": "string"}, "pid": {"type": ["integer", "null"]}, "url": {"type": ["string", "null"]}, "opened": {"type": "boolean"}})),
            "annotations": annotations(false, false),
        },
        {
            "name": "game_demo",
            "title": "Watch a game play itself",
            "description": "OPT-IN (GAMECRAFT_MCP_ALLOW_LAUNCH=1). Like game_play, but the game plays itself: web games get ?autoplay=1, Godot games GAMECRAFT_AUTOPLAY=1, Bevy games --autoplay. Use to show what a game is without asking the user to play it.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list."},
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"]},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }, "required": ["slug"]},
            "outputSchema": envelope(&json!({"slug": {"type": "string"}, "pid": {"type": ["integer", "null"]}, "url": {"type": ["string", "null"]}, "opened": {"type": "boolean"}})),
            "annotations": annotations(false, false),
        },
        {
            "name": "game_running",
            "title": "What is running",
            "description": "Native games (Godot, Bevy) started from this machine that are still alive, with pid, mode and start time, and whether the web arcade answers on its port. Web games run in a browser tab and are not listed. Read-only.",
            "inputSchema": {"type": "object", "properties": {
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }},
            "outputSchema": envelope(&json!({"games": {"type": "array"}, "arcade_up": {"type": "boolean"}, "arcade_pid": {"type": ["integer", "null"]}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_stop",
            "title": "Stop games",
            "description": "OPT-IN (GAMECRAFT_MCP_ALLOW_LAUNCH=1). Stop the native game named by slug, or all of them, that this tool started. server=true also stops the web arcade server if this tool started it. Web games cannot be stopped from here: the user closes the tab.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Game to stop; omit to stop every native game started from here."},
                "server": {"type": "boolean", "default": false, "description": "Also stop the arcade server this tool started."},
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }},
            "outputSchema": envelope(&json!({"stopped": {"type": "array"}})),
            "annotations": annotations(false, true),
        },
        {
            "name": "game_doctor",
            "title": "Catalog and tool health check",
            "description": "Report-only: games that cannot be started (data or plans without code), games that never passed or now fail their gate, missing cheat sheets, descriptions or thumbnails, plans with no game yet, and whether bun, node, cargo and godot are installed. Changes nothing. Returns failing, warnings, planned and findings[{slug, level, issue}].",
            "inputSchema": {"type": "object", "properties": {
                "root": {"type": "string", "description": "Repo root; found automatically when omitted."}
            }},
            "outputSchema": envelope(&json!({"failing": {"type": "integer"}, "warnings": {"type": "integer"}, "planned": {"type": "integer"}, "findings": {"type": "array"}})),
            "annotations": annotations(true, false),
        },
        {
            "name": "game_verify",
            "title": "Run a game's gate",
            "description": "OPT-IN: run one game's verification gate now and return the fresh result. Web games run the headless-Chromium self-play gate (needs GAMECRAFT_MCP_ALLOW_WEB=1, node and bun; rewrites verify.json and shots/). Godot games boot headless Godot and look for a win marker (needs GAMECRAFT_MCP_ALLOW_GODOT=1). Takes 10 s to a few minutes. Returns pass plus the gate record.",
            "inputSchema": {"type": "object", "properties": {
                "slug": {"type": "string", "description": "Folder name from game_list."},
                "kind": {"type": "string", "enum": ["godot", "web", "bevy"]},
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
    ]);
    registry_tools(&mut tools);
    tools
}

/// Tools generated from the command registry: the replay commands, plus the three that
/// expose the registry itself (list, describe, run), so any command is reachable even
/// from a client that only reads the tool list once.
fn registry_tools(tools: &mut Value) {
    let Some(list) = tools.as_array_mut() else {
        return;
    };
    for c in registry::COMMANDS
        .iter()
        .filter(|c| c.id.starts_with("replay."))
    {
        let mut props = serde_json::Map::new();
        let mut required = Vec::new();
        for a in c.params {
            let mut schema = json!({"type": a.kind, "description": a.help});
            if !a.choices.is_empty() {
                schema["enum"] = json!(a.choices);
            }
            props.insert(a.name.to_string(), schema);
            if a.required {
                required.push(a.name);
            }
        }
        list.push(json!({
            "name": registry::tool_name(c.id),
            "title": c.label,
            "description": format!("{} Command id {}.", c.summary, c.id),
            "inputSchema": {"type": "object", "properties": props, "required": required},
            "outputSchema": envelope(&json!({})),
            "annotations": annotations(c.readonly, false),
        }));
    }
    list.push(json!({
        "name": "list_commands",
        "title": "List commands",
        "description": "Every command in the GameCraft registry with its id, menu path, shortcut and typed parameters. The same list `gamecraft commands --json` prints.",
        "inputSchema": {"type": "object", "properties": {}},
        "outputSchema": envelope(&json!({"count": {"type": "integer"}, "commands": {"type": "array", "items": {"type": "object"}}})),
        "annotations": annotations(true, false),
    }));
    list.push(json!({
        "name": "describe_command",
        "title": "Describe a command",
        "description": "One command: summary, menu, shortcut, whether it is read-only, and its parameters with types and allowed values.",
        "inputSchema": {"type": "object", "properties": {"id": {"type": "string", "description": "Command id, e.g. replay.verify."}}, "required": ["id"]},
        "outputSchema": envelope(&json!({})),
        "annotations": annotations(true, false),
    }));
    list.push(json!({
        "name": "run_command",
        "title": "Run a command",
        "description": "Run any registered command by id with a params object. Same code path as the dedicated tools and as `gamecraft run --cmd id={json}`.",
        "inputSchema": {"type": "object", "properties": {
            "id": {"type": "string", "description": "Command id, e.g. game.list."},
            "params": {"type": "object", "description": "The command parameters; see describe_command."}
        }, "required": ["id"]},
        "outputSchema": envelope(&json!({})),
        "annotations": annotations(false, false),
    }));
}

/// The meta tools: list, describe and run commands from the registry.
fn registry_meta(name: &str, args: &Value) -> Option<ToolOutcome> {
    match name {
        "list_commands" => Some(Ok((
            format!("{} commands.", registry::COMMANDS.len()),
            registry::list_json(),
        ))),
        "describe_command" => Some(
            match args
                .get("id")
                .and_then(Value::as_str)
                .and_then(registry::describe)
            {
                Some(d) => Ok((format!("Command {}.", d["id"].as_str().unwrap_or("")), d)),
                None => Err(fail(
                    "unknown_command",
                    "No such command.",
                    "Call list_commands for the ids.",
                    "list_commands",
                )),
            },
        ),
        "run_command" => {
            let id = args.get("id").and_then(Value::as_str).unwrap_or("");
            let params = args.get("params").cloned().unwrap_or_else(|| json!({}));
            Some(registry::run(id, &params))
        }
        _ => None,
    }
}

// ------------------------------------------------------------------ results

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

/// True for the three meta tools that front the registry.
fn registry_meta_name(name: &str) -> bool {
    matches!(name, "list_commands" | "describe_command" | "run_command")
}

// ------------------------------------------------------------------ resources

/// Resource URI of a game's cheat sheet.
fn sheet_uri(kind: &str, slug: &str) -> String {
    format!("gamecraft://games/{kind}/{slug}/cheatsheet")
}

/// Every cheat sheet in the catalog, as `gamecraft://games/<kind>/<slug>/cheatsheet`.
fn resource_list() -> Value {
    let games = catalog::find_root(None)
        .map(|root| catalog::discover(root.as_path()))
        .unwrap_or_default();
    let resources: Vec<Value> = games
        .iter()
        .filter(|g| g.dir.join("CHEATSHEET.md").is_file())
        .map(|g| {
            json!({
                "uri": sheet_uri(g.kind.as_str(), g.slug.as_str()),
                "name": format!("{} cheat sheet", g.title),
                "description": format!("Goal, controls and gate for the {} game {}.", g.kind.as_str(), g.slug),
                "mimeType": "text/markdown",
            })
        })
        .collect();
    json!({ "resources": resources })
}

/// Read one cheat sheet by URI.
fn resource_read(uri: &str) -> Result<Value, (i32, String)> {
    let not_found = || (-32002_i32, format!("no such resource: {uri}"));
    let rest = uri
        .strip_prefix("gamecraft://games/")
        .ok_or_else(not_found)?;
    let mut parts = rest.split('/');
    let (Some(kind), Some(slug), Some("cheatsheet"), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(not_found());
    };
    let root = catalog::find_root(None).ok_or_else(not_found)?;
    let games = catalog::discover(root.as_path());
    let game = catalog::find(&games, slug, catalog::GameKind::parse(kind)).ok_or_else(not_found)?;
    let text = std::fs::read_to_string(game.dir.join("CHEATSHEET.md")).map_err(|_| not_found())?;
    Ok(json!({ "contents": [{ "uri": uri, "mimeType": "text/markdown", "text": text }] }))
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
        "game_describe" => tool_game_describe(args),
        "game_help" => tool_game_help(args),
        "game_start" => tool_game_start(args, None),
        "game_play" => tool_game_start(args, Some(Mode::Play)),
        "game_demo" => tool_game_start(args, Some(Mode::Demo)),
        "game_running" => tool_game_running(args),
        "game_stop" => tool_game_stop(args),
        "game_doctor" => tool_game_doctor(args),
        _ if registry::id_for_tool(name).is_some() || registry_meta_name(name) => {
            registry_meta(name, args)
                .unwrap_or_else(|| registry::run(registry::id_for_tool(name).unwrap_or(name), args))
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, args: &Value) -> Value {
        call_tool(name, args).unwrap_or_else(|(c, m)| json!({"rpc_error": c, "message": m}))
    }

    #[test]
    fn registry_tools_are_listed_and_run_the_same_code_as_run_command() {
        let names: Vec<String> = tool_list()
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        for n in [
            "replay_record",
            "replay_verify",
            "replay_info",
            "list_commands",
            "describe_command",
            "run_command",
        ] {
            assert!(names.iter().any(|x| x == n), "{n} missing from tools/list");
        }
        let listed = call("list_commands", &json!({}));
        assert_eq!(
            listed.pointer("/structuredContent/count"),
            Some(&json!(registry::COMMANDS.len()))
        );
        // The dedicated tool and run_command give the identical answer.
        let direct = call("replay_info", &json!({"path": "nope.txt"}));
        let via = call(
            "run_command",
            &json!({"id": "replay.info", "params": {"path": "nope.txt"}}),
        );
        assert_eq!(direct, via);
        assert_eq!(
            direct.pointer("/structuredContent/error"),
            Some(&json!("wrong_extension"))
        );
        let unknown = call("run_command", &json!({"id": "replay.rec"}));
        assert_eq!(unknown["isError"], true);
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
        assert_eq!(list.len(), 23);
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
