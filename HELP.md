# GameCraft Help & Operator Guide

**Describe a game in a sentence. Get a deterministic, verified Godot 4 project (or play the Godot, three.js, and Bevy libraries).**

## 1. Get Started

1. **Check Your Environment**: Run `gamecraft-cli doctor` (or `cargo run -p gamecraft-cli -- doctor`) to verify Godot 4.4, Ollama, Bun, and workspace paths.
2. **Browse the Game Library**: Run `gamecraft-cli list` (or open `gamecraft-desktop`) to see all Godot (`depot/games/`), three.js (`web/games/`), and Bevy (`bevy/games/`) games along with their last verification gate status.
3. **Play or Watch a Demo**: Run `gamecraft-cli play <slug>` to launch a game, `gamecraft-cli demo <slug>` to watch a recorded run, or `gamecraft-cli help <slug>` for a game's full controls and cheat sheet.
4. **Generate a New Game from a Prompt**: Run:
   ```powershell
   gamecraft-cli new --prompt "a snowy mountain pass at dusk, hard jumps" --provider local
   ```
   GameCraft prompts a local Ollama model (e.g. `gemma4:12b`) to emit a structured JSON `spec.json` (never raw GDScript), validates it, compiles a deterministic Godot 4 scene, lints it, replays it in Rust, and boots Godot headless to verify a win.
5. **Compile Your Own Spec Without an LLM**: Run `gamecraft-cli new --from-spec depot/games/snowy-mountain-pass/spec.json` to bypass the model completely.

## 2. Functions & Architecture

- **Spatial Compiler (`gamecraft-core`, `gamecraft-commands`, `gamecraft-godot`)**: WASM-safe scene graph, undoable command bus, and deterministic `nom` parser/emitter for Godot 4 `.tscn`/`.tres` files.
- **Deterministic Simulation & Replay (`gamecraft-sim`, `gamecraft-replay`)**: Headless kinematic physics stepper, BFS maze/crate solvers, and hash-checked `.gcraft` replay logs.
- **Depot Forge & 3D Export (`gamecraft-forge`, `gamecraft-gltf`, `gamecraft-trellis`)**: Builds the Godot depot games from verified traces, exports deterministic glTF 2.0 scenes, and provides a thin plan-only TRELLIS.2 sidecar client (never bundles the 4B model).
- **Multi-Runtime Playback (`gamecraft-bevy`, `web/`, Godot 4.4)**: Native Bevy 0.20 sims (`cargo run --profile play`), three.js web arcade (`cd web; bun run serve` on port `11258`), and Godot 4 projects.
- **CLI, Desktop Viewer & MCP (`gamecraft-cli`, `gamecraft-desktop`, `gamecraft-mcp`)**: Unified CLI, native egui desktop viewer, and hand-rolled `stdio` MCP server (`game_list`, `game_describe`, `game_help`, `game_report`, `game_running`, `game_doctor`, `scene_lint`, `spatial_query`, `scene_inspect`, `export_scene`).

## 3. Usage (GUI, CLI & MCP)

- **Desktop Viewer**: `cargo run -p gamecraft-desktop` — browse the library, inspect verification status, and launch games with one click.
- **CLI**:
  ```powershell
  gamecraft-cli doctor
  gamecraft-cli list
  gamecraft-cli help <slug>
  gamecraft-cli new --prompt "night forest, hard jumps" --provider local
  gamecraft-cli new --from-spec depot/games/snowy-mountain-pass/spec.json
  ```
- **MCP (`stdio`)**: `cargo run -p gamecraft-mcp` — exposes scene inspection/lint/export and game catalog/help/verify tools over pure JSON-RPC `stdio`.

## 4. Tips & Tricks

- **Use `gemma4:12b` for Local Generation**: Known working local Ollama model; avoid `qwen3.8:27b` (known Ollama socket reset crash).
- **8 Supported Prompt Archetypes**: `platformer`, `runner`, `pong`, `maze`, `flight`, `driving`, `chess`, `talk`. Unsupported archetypes are explicitly rejected with a list of supported ones rather than silently falling back.
- **No Godot Needed for `new` Compilation**: Compiling, linting, and Rust-replaying a spec requires neither Godot nor a GPU; Godot 4.4 is only needed for in-engine headless win verification or editing `project.godot`.
- **Fast Bevy Builds**: Use `--profile play` so Bevy/wgpu dependencies compile at `opt-level = 3` while local code stays fast to iterate at `opt-level = 1`.

## 5. Common Pitfalls & Safety

- **Zero-Unwrap & `unsafe_code = "forbid"`**: Workspace enforces `#![forbid(unsafe_code)]`, `clippy::pedantic = "deny"`, and denies `unwrap_used`, `expect_used`, `panic`, `todo`, and `unimplemented`.
- **Honest Verification Gates**: `just verify-depot` keeps unproven depot games visibly red (4 of 15 long-standing games — `breakout`, `flappy`, `topdown-shooter`, `tower-place` — boot cleanly but do not prove an in-engine win yet).
- **Cloud LLM Providers**: OpenAI/Anthropic paths are wired and stub-tested only; local Ollama is the live-verified path.

## 6. Crate & App Documentation

- L1 Core & Compilers: [gamecraft-core](crates/gamecraft-core/README.md), [gamecraft-commands](crates/gamecraft-commands/README.md), [gamecraft-godot](crates/gamecraft-godot/README.md), [gamecraft-sim](crates/gamecraft-sim/README.md), [gamecraft-replay](crates/gamecraft-replay/README.md), [gamecraft-forge](crates/gamecraft-forge/README.md), [gamecraft-gltf](crates/gamecraft-gltf/README.md), [gamecraft-trellis](crates/gamecraft-trellis/README.md)
- L2/L3 Runtimes & Apps: [gamecraft-mcp](crates/gamecraft-mcp/README.md), [gamecraft-bevy](crates/gamecraft-bevy/README.md), [gamecraft-cli](apps/gamecraft-cli/README.md), [gamecraft-desktop](apps/gamecraft-desktop/README.md)
