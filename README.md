# GameCraft

Pure-Rust spatial compiler, scene verification engine, and MCP server for Godot 4.x.

- **L1 `gamecraft-core`**: node graph, `glam` transform hierarchy, AABB aggregation,
  collision matrix, signal registry. WASM-safe (no `wgpu`/`egui`/godot deps).
- **L1 `gamecraft-commands`**: undoable `Command` trait (`execute`/`undo`/`serialize_json`).
- **L1 `gamecraft-godot`**: deterministic `nom` parser for Godot 4 `.tscn`/`.tres` text
  plus deterministic UID hashing from the relative scene path.
- **L2 `gamecraft-mcp`**: stdio MCP server (`scene_lint`, `spatial_query`).
- **L3 `gamecraft-cli`**: `lint` / `query` / `uid` / `mcp` subcommands.
- **L3 `gamecraft-desktop`**: `egui`/`wgpu` shell placeholder (viewport lands in Phase 4).

## Quickstart

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p gamecraft-cli -- lint --scene apps/gamecraft-cli/sample.tscn
```

See `AGENTS.md` for gates and `CHANGELOG.md` for history.
