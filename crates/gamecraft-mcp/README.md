# `gamecraft-mcp` (L2 Hand-Rolled MCP Stdio Server)

Dual-era Model Context Protocol (`stdio`) server for **GameCraft** (supports both `2026-07-28` stateless and `initialize`-handshake `2025-11-25..2026-07-28` clients).

## Exposed Tools

- **Game Library & Guide**: `game_list`, `game_describe`, `game_help`, `game_report`, `game_running`, `game_doctor`, `game_start`, `game_stop`.
- **Scene Inspection & Export**: `scene_lint`, `spatial_query`, `scene_inspect`, `export_scene`.

```powershell
cargo run -p gamecraft-mcp
```
