//! `gamecraft-mcp`: hand-rolled MCP `2024-11-05` stdio server.
//!
//! Newline-delimited JSON-RPC on stdin/stdout; logs go to stderr so stdout
//! stays 100% JSON. Tools: `scene_lint`, `spatial_query`.

use gamecraft_mcp::{scene_lint, spatial_query};
use std::io::{BufRead, Write};

/// MCP protocol version spoken here.
const PROTOCOL_VERSION: &str = "2024-11-05";

fn main() {
    let stdin = std::io::stdin();
    let lines = stdin.lock().lines();
    for line in lines {
        let Ok(text) = line else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(&text) {
            let mut stdout = std::io::stdout();
            if writeln!(stdout, "{reply}").is_err() {
                break;
            }
            if stdout.flush().is_err() {
                break;
            }
        }
    }
}

/// Handle one JSON-RPC message; `None` = notification, no reply.
fn handle(text: &str) -> Option<String> {
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    let method = msg.get("method")?.as_str()?.to_string();
    let id = msg.get("id").cloned()?;
    let result = dispatch(method.as_str(), &msg);
    let reply = match result {
        Ok(value) => serde_json::json!({"jsonrpc": "2.0", "id": id, "result": value}),
        Err((code, message)) => serde_json::json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": code, "message": message},
        }),
    };
    serde_json::to_string(&reply).ok()
}

/// Route one method call; `Err((code, message))` becomes a JSON-RPC error.
fn dispatch(method: &str, msg: &serde_json::Value) -> Result<serde_json::Value, (i32, String)> {
    match method {
        "initialize" => Ok(serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "gamecraft-mcp", "version": env!("CARGO_PKG_VERSION")},
        })),
        "tools/list" => Ok(serde_json::json!({
            "tools": [
                {
                    "name": "scene_lint",
                    "description": "Lint Godot 4 .tscn/.tres text: missing collision shapes, unresolvable signal targets/methods.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scene_text": {"type": "string", "description": "Full .tscn/.tres source text."},
                            "scene_file": {"type": "string", "description": "Path to a .tscn. res:// requires root."},
                            "root": {"type": "string", "description": "Absolute project directory, used to resolve res:// and to confine reads."}
                        },
                        "required": ["scene_text"],
                    },
                },
                {
                    "name": "spatial_query",
                    "description": "World-space AABB of one node subtree.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scene_text": {"type": "string"},
                            "node_path": {"type": "string", "description": "Scene path from the root, or '.' for the root."}
                        },
                        "required": ["scene_text", "node_path"],
                    },
                },
                {
                    "name": "scene_inspect",
                    "description": "Read-only views of a parsed scene. operation='tree' (paginated node list), 'signals' (connections with resolved/reason), 'property' (one node's transform+properties), 'find' (substring over names and types).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scene_text": {"type": "string"},
                            "scene_file": {"type": "string", "description": "Path to a .tscn. res:// requires root."},
                            "root": {"type": "string", "description": "Absolute project directory, used to resolve res:// and to confine reads."},
                            "operation": {"type": "string", "enum": ["tree", "signals", "property", "find"], "default": "tree"},
                            "node_path": {"type": "string", "description": "Required for operation='property'."},
                            "query": {"type": "string", "description": "Required for operation='find'."},
                            "depth": {"type": "integer", "default": 64},
                            "limit": {"type": "integer", "default": 100},
                            "offset": {"type": "integer", "default": 0}
                        }
                    },
                },
                {
                    "name": "export_scene",
                    "description": "Deterministically export a JSON scene spec to an engine format. format='godot4' writes Godot 4 .tscn text (the export_godot path). Other targets (e.g. 'overte') are format values on this same tool, never new tools.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "format": {"type": "string", "enum": ["godot4"], "default": "godot4"},
                            "scene": {
                                "type": "object",
                                "description": "Scene spec: {uid?, format?, nodes[], connections[], ext_resources[], sub_resources[]}. Node: {name, type, parent?, unique?, position?, rotation_degrees?, scale?, properties?}. A property value is a JSON scalar, a 3-number array (Vector3), or {\"raw\": \"<godot syntax>\"}.",
                                "required": ["nodes"],
                                "properties": {
                                    "uid": {"type": "string"},
                                    "format": {"type": "integer"},
                                    "nodes": {"type": "array", "items": {"type": "object"}},
                                    "connections": {"type": "array", "items": {"type": "object"}},
                                    "ext_resources": {"type": "array", "items": {"type": "object"}},
                                    "sub_resources": {"type": "array", "items": {"type": "object"}}
                                }
                            }
                        },
                        "required": ["scene"],
                    },
                },
            ],
        })),
        "tools/call" => {
            let params = msg
                .get("params")
                .ok_or_else(|| (-32602_i32, "missing params".to_string()))?;
            let name = params
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| (-32602_i32, "missing tool name".to_string()))?;
            let args = params
                .get("arguments")
                .ok_or_else(|| (-32602_i32, "missing arguments".to_string()))?;
            call_tool(name, args)
        }
        "ping" => Ok(serde_json::json!({})),
        _ => Err((-32601, format!("method not found: {method}"))),
    }
}

/// Run one tool and wrap its JSON output in MCP content form.
fn call_tool(name: &str, args: &serde_json::Value) -> Result<serde_json::Value, (i32, String)> {
    let payload = match name {
        "scene_lint" => {
            let text = scene_text_arg(args)?;
            serde_json::to_value(scene_lint(text.as_str()))
                .map_err(|err| (-32603_i32, format!("encode error: {err}")))?
        }
        "spatial_query" => {
            let text = scene_text_arg(args)?;
            spatial_query(text.as_str(), string_arg(args, "node_path")?)
                .map_err(|reason| (-32603_i32, reason))?
        }
        "scene_inspect" => {
            let text = scene_text_arg(args)?;
            let options = gamecraft_mcp::InspectOptions {
                operation: args
                    .get("operation")
                    .and_then(|value| value.as_str())
                    .unwrap_or(gamecraft_mcp::OP_TREE),
                node_path: args.get("node_path").and_then(|value| value.as_str()),
                query: args.get("query").and_then(|value| value.as_str()),
                depth: usize_arg(args, "depth", gamecraft_mcp::DEFAULT_DEPTH),
                limit: usize_arg(args, "limit", gamecraft_mcp::DEFAULT_LIMIT),
                offset: usize_arg(args, "offset", 0),
            };
            gamecraft_mcp::scene_inspect(text.as_str(), &options)
                .map_err(|reason| (-32603_i32, reason))?
        }
        "export_scene" => {
            let format = args
                .get("format")
                .and_then(|value| value.as_str())
                .unwrap_or(gamecraft_mcp::spec::FORMAT_GODOT4);
            let scene = args
                .get("scene")
                .ok_or_else(|| (-32602_i32, "missing scene".to_string()))?;
            let spec: gamecraft_mcp::SceneSpec = serde_json::from_value(scene.clone())
                .map_err(|err| (-32602_i32, format!("invalid scene spec: {err}")))?;
            let outcome =
                gamecraft_mcp::export_spec(&spec, format).map_err(|reason| (-32603_i32, reason))?;
            serde_json::json!({
                "format": format,
                "uid": outcome.uid,
                "nodes": outcome.nodes,
                "bytes": outcome.text.len(),
                "text": outcome.text,
            })
        }
        _ => return Err((-32601, format!("unknown tool: {name}"))),
    };
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": payload.to_string()}],
    }))
}

/// Resolve `scene_text` / `scene_file` (+`root`) into scene source.
fn scene_text_arg(args: &serde_json::Value) -> Result<String, (i32, String)> {
    let text = args.get("scene_text").and_then(|value| value.as_str());
    let file = args.get("scene_file").and_then(|value| value.as_str());
    let root = args.get("root").and_then(|value| value.as_str());
    gamecraft_mcp::resolve_scene_text(text, file, root).map_err(|reason| (-32602_i32, reason))
}

/// Read an optional unsigned argument.
fn usize_arg(args: &serde_json::Value, key: &str, fallback: usize) -> usize {
    args.get(key)
        .and_then(serde_json::Value::as_u64)
        .map_or(fallback, |value| usize::try_from(value).unwrap_or(fallback))
}

/// Extract one required string argument.
fn string_arg<'a>(args: &'a serde_json::Value, key: &str) -> Result<&'a str, (i32, String)> {
    args.get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| (-32602_i32, format!("missing {key}")))
}
