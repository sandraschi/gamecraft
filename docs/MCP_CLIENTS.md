# Connecting GameCraft MCP to a client

The server is a single stdio process. No port, no daemon, no auth:

```
D:\Dev\repos\gamecraft\target\release\gamecraft-mcp.exe
```

Build it with `cargo build --release -p gamecraft-mcp`. JSON-RPC goes on stdout,
logs on stderr, so it is safe as a child process of any client.

## Where each client reads its config (verified on this machine)

| Client | File | Root key | Entry shape |
|---|---|---|---|
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | `mcpServers` | `{command, args}` |
| Claude Code (project) | `<repo>/.mcp.json` | `mcpServers` | `{command, args}` |
| opencode | `%USERPROFILE%\.config\opencode\opencode.json` | `mcp` | `{type:"local", command:[...], enabled}` |
| Cursor | `%USERPROFILE%\.cursor\mcp.json` | `mcpServers` | `{command, args}` |
| Antigravity | `%USERPROFILE%\.gemini\antigravity\mcp_config.json` | `mcpServers` | `{command, args}` |

Antigravity's file was empty before this was added, so its root key is the
shared `mcpServers` shape rather than something read out of a working file. If
the IDE's MCP panel does not list it, add the same path there by hand.

### Claude Desktop / Cursor / Antigravity

```json
{
  "mcpServers": {
    "gamecraft": {
      "command": "D:\\Dev\\repos\\gamecraft\\target\\release\\gamecraft-mcp.exe",
      "args": []
    }
  }
}
```

### opencode

```json
{
  "mcp": {
    "gamecraft": {
      "type": "local",
      "command": ["D:\\Dev\\repos\\gamecraft\\target\\release\\gamecraft-mcp.exe"],
      "enabled": true
    }
  }
}
```

Restart the client after editing (Claude Desktop and Cursor read MCP config only
at startup; opencode re-reads on new session).

## Tools

| Tool | Arguments | Returns |
|---|---|---|
| `scene_lint` | `scene_text` | Array of findings. Codes: `ERR_MISSING_COLLISION_SHAPE`, `ERR_UNRESOLVED_SIGNAL_TARGET`, `ERR_UNRESOLVED_RESOURCE`, `ERR_SCENE_PARSE`. Empty array = clean. |
| `spatial_query` | `scene_text`, `node_path` | World-space AABB of a subtree: `{node, empty, min, max}`. |
| `export_scene` | `format` (`godot4`), `scene` | Deterministic `.tscn` text: `{format, uid, nodes, bytes, text}`. |

`export_scene` takes format as a parameter on purpose: a second target (Overte,
glTF, another engine) is a new enum value, not a new tool in the client's
context window.

Scene spec shape:

```json
{
  "uid": "uid://optional",
  "nodes": [
    {"name": "World", "type": "Node3D", "properties": {"script": {"raw": "ExtResource(\"1_w\")"}}},
    {"name": "Body", "type": "StaticBody3D", "parent": ".", "position": [0, 0.5, 0]},
    {"name": "Shape", "type": "CollisionShape3D", "parent": "Body",
     "properties": {"shape": {"raw": "SubResource(\"box\")"}}}
  ],
  "connections": [{"signal": "body_entered", "from": "Goal", "to": ".", "method": "_on_goal"}],
  "ext_resources": [{"type": "Script", "path": "res://scripts/world.gd", "id": "1_w"}],
  "sub_resources": [{"type": "BoxShape3D", "id": "box", "properties": {"size": "Vector3(1, 1, 1)"}}]
}
```

A property value is a JSON scalar, a 3-number array (Vector3), or `{"raw": "..."}`
for anything that is already Godot syntax (`ExtResource`, `NodePath`, `Array[...]`).

## Prompts that work

Audit something you already have:

- "Lint `depot/games/platformer/main.tscn` and tell me every finding, grouped by node."
- "Here is my scene file. Lint it, then give me the smallest patch that clears each finding."
- "Which physics bodies in this scene are missing collision shapes, and which ones are nested under a parent body (so the child's shape already covers them)?"

Reason about space:

- "Give me the world AABB of `Player` and of `World/Crate`, then tell me whether they can overlap after any transform."
- "Compare the AABB of every `CollisionShape3D` against its parent body's mesh and flag shapes that are visibly larger or smaller than what they represent."

Generate:

- "Export a minimal Godot 4 scene: a `Node3D` root, a current `Camera3D`, one `StaticBody3D` with a `BoxMesh`, a `BoxShape3D`, and a `CharacterBody3D` marked unique. Then lint the result and fix anything it reports."
- "Build me a 5-room dungeon as a scene spec: each room a `StaticBody3D` floor, doors as `Area3D` triggers wired to `_on_door`, and every door reachable from the entrance. Lint before returning."

Round-trip discipline:

- "Export this spec, lint the output, and if lint is non-empty, iterate until it is empty. Show me the final `.tscn` and the diff from the first attempt."
- "Take this `.tscn`, describe it as a JSON scene spec, then re-export it and diff against the original. I want to know what the round trip loses."

## What the tool will not do

- It does not run Godot. `scene_lint` is static analysis; a clean scene still has
  to boot in the editor. The repo's own gate for that is `just depot-godot`.
- It does not invent `uid://` values. Omitted or unrecognised uid is a
  deterministic hash of the file path, so the same scene always exports the same
  uid.
- It does not accept arbitrary Godot syntax in typed fields: a string property is
  a string. Use `{"raw": ...}` when you mean expression syntax, and the export
  is then byte-identical on every run.
