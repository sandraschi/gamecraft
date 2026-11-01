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

## Protocol and result contract

- Dual-era. **Modern** `2026-07-28` (stateless): no handshake, the version
  rides in each request's `_meta["io.modelcontextprotocol/protocolVersion"]`;
  `server/discover` lists versions, capabilities and `instructions`; results
  carry `resultType: "complete"` and `_meta["io.modelcontextprotocol/serverInfo"]`;
  `tools/list` carries `ttlMs` (1 h) and `cacheScope: "public"`. An unknown
  version gets error `-32022` with `data.supported`. **Legacy** `2025-11-25`,
  `2025-06-18`, `2025-03-26`, `2024-11-05`: the `initialize` handshake, which
  returns `instructions` and negotiates to the client's revision (anything
  unknown gets `2025-11-25`).
- Every tool has a `title`, `annotations` (`readOnlyHint`, `destructiveHint`,
  `idempotentHint`, `openWorldHint: false`) and an `outputSchema`.
- A result's first text block is one plain sentence ("28 games (15 Godot, 13
  web); none failing."); the second is the JSON. The same data is in
  `structuredContent` with `success` and `message`.
- A failed tool call is a **result** with `isError: true`, not a JSON-RPC
  error, so the model sees it and can recover. Shape (fleet dialogic returns):
  `{success: false, error, message, dialogic: {suggestion, remediation}}`, and
  the text reads "what went wrong / What to do / Next: <tool call>". Example:
  `game_report {"slug":"benny"}` answers "There is no game 'benny'." with
  `Next: game_report {"slug": "where-is-benny"}`.
- Only protocol faults (unknown tool, malformed JSON) are JSON-RPC errors.

## Environment

| Variable | Effect |
|---|---|
| `GAMECRAFT_ROOT` | Repo root for the catalog tools. Default: walk up from the working directory and the executable looking for `depot/games` or `web/games`. |
| `GAMECRAFT_MCP_ALLOW_WEB=1` | Lets `game_verify` run the three.js gate (headless browser, up to minutes). Off by default. |
| `GAMECRAFT_MCP_ALLOW_GODOT=1` | Lets `game_verify` and `scene_verify` boot Godot. Off by default. |
| `GAMECRAFT_GODOT` | Godot binary (default `godot`). |

## Tools

Scene tools (read-only; each takes `scene_text` or `scene_file`, plus `root` for `res://` paths):

| Tool | Arguments | Returns |
|---|---|---|
| `scene_lint` | scene | Array of findings. Codes: `ERR_MISSING_COLLISION_SHAPE`, `ERR_UNRESOLVED_SIGNAL_TARGET`, `ERR_UNRESOLVED_RESOURCE`, `ERR_SCENE_PARSE`. Empty array = clean. |
| `spatial_query` | scene, `node_path` | World-space AABB of a subtree: `{node, empty, min, max}`. |
| `scene_inspect` | scene, `operation` (`tree`/`signals`/`property`/`find`), `node_path`, `query`, `offset` | Paginated node tree, every signal with resolved/reason, one node's properties, or a name/type search. |
| `export_scene` | `format` (`godot4` or `gltf`), `scene` | Deterministic text: `{format, uid, nodes, bytes, text}`. `godot4` gives `.tscn`; `gltf` gives glTF 2.0 JSON with the buffer embedded as base64 (primitive meshes, PBR colours, lights, camera; no collision shapes or scripts). |

Catalog tools (the same catalog `gamecraft-cli list` and the desktop library read):

| Tool | Arguments | Returns |
|---|---|---|
| `game_list` | `filter?`, `kind?` (`godot`/`web`), `root?` | Every game with kind, group, title, description, last gate result. |
| `game_report` | `slug`, `kind?`, `root?` | One game: gate record, cheat sheet, README, screenshot paths. Unknown slugs get suggestions. |
| `game_verify` | `slug`, `kind?`, `timeout_s?` (300), `root?` | Runs the gate now (web: headless self-play with checkpoints; Godot: headless boot to a win marker). Opt-in, see Environment. |
| `game_create` | `spec?`, `explain?`, `root?` | `explain: true` returns the GameSpec guide; with a `spec` it validates and writes a new Godot game. The only tool that writes. |
| `scene_verify` | `game_path`, `frames?`, `autoplay?`, `timeout_s?` | Boots a Godot project headless and reports the trace. Opt-in. |

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
