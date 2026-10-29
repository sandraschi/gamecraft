# GameCraft

Pure-Rust spatial compiler, scene verification engine, and MCP server for Godot 4.x —
plus a 15-game generated depot and a desktop viewer that renders it.

Games are **compiled from a `GameSpec`**, not hand-written in GDScript. The spec is
small, validated, and deterministic; the same spec always produces byte-identical
scenes, which is what makes the depot gate possible.

## Crates

| Crate | Layer | Role |
|---|---|---|
| `gamecraft-core` | L1 | Node graph, `glam` transform hierarchy, AABB aggregation, collision matrix, signal registry. **No `wgpu`/`egui`/godot deps; compiles to `wasm32-unknown-unknown`.** |
| `gamecraft-commands` | L1 | Undoable `Command` trait and stack (`AddNode`, `ReparentNode`, `SetProperty`, `SetTransform`). |
| `gamecraft-godot` | L1 | Deterministic `nom` parser **and serializer** for Godot 4 `.tscn`/`.tres`, deterministic UID hashing, and the scene lint rules. |
| `gamecraft-sim` | L1 | Deterministic gameplay sims: kinematic stepper, BFS maze and crate-push solvers, mirrored Rust trace replays. |
| `gamecraft-forge` | L1 | Depot forge: game builders, `GameSpec` validation, the local/cloud LLM front end, software rasterizer for `thumb.png`/`play.gif`, and the Phase 8 economy/Steam asset generators. |
| `gamecraft-mcp` | L2 | Hand-rolled MCP `2024-11-05` stdio server: `scene_lint`, `spatial_query`, `export_scene`, `scene_inspect`. |
| `gamecraft-cli` | L3 | `clap` CLI: `lint`, `query`, `uid`, `forge`, `mcp`. |
| `gamecraft-desktop` | L3 | `egui`/`wgpu` viewer: depot browser with Start Game, shaded mesh viewport with scene lighting, scene tree with lint badges, inspector, trace scrubber, command palette with undo/redo. |

## Quickstart

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# gates
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo check -p gamecraft-core --target wasm32-unknown-unknown

# rebuild the whole depot (15 games) and boot-check each one
cargo run -p gamecraft-cli -- forge --all
just godot-import          # Godot 4.4 needs --editor --quit-after; --headless --import is broken

# viewer
cargo run -p gamecraft-desktop

# MCP over stdio
cargo run -p gamecraft-mcp
```

## Controls

**In the games** (Godot): arrow keys or WASD to move, Space/Enter to jump, flap
or confirm.

**Gamepad.** Godot's built-in `ui_*` actions already carry joypad bindings for
movement (D-pad buttons 11–14 and left-stick axes 0/1), but `ui_accept` ships
**keys-only** — so a pad could walk and never jump. The shared runtime adds the
missing buttons at startup, idempotently, rather than re-declaring input maps in
15 `project.godot` files:

| Input | Effect |
|---|---|
| D-pad / left stick | move, walk, drive, chess cursor |
| **A** | jump, flap, confirm knight move |
| **B** | quit the game (there is no menu to go back to) |

Verified by probing `InputMap.action_get_events` — `ui_accept` reported
`KEY, KEY, KEY` before this change.

**Desktop viewer** (`gilrs`, polled once per frame):

| Input | Effect |
|---|---|
| Left stick horizontal | orbit the viewport camera |
| D-pad / left-stick vertical | move the depot selection (with hold-repeat) |
| **A** | start the highlighted depot game |
| **B** | return selection to the scene root |

Keyboard and mouse remain fully functional; a missing pad degrades to "not
connected" and never blocks the shell.

## Depot

`depot/games/<slug>/` holds 15 standalone Godot 4.4 projects (13 forged +
`night-forest` and `snowy-mountain-pass` generated from specs). Each folder carries
`project.godot`, `main.tscn`, `scripts/`, `lint.json`, `trace.json`, `commands.jsonl`,
`thumb.png`, `play.gif`, `README.md`. The two spec-generated entries also carry
`spec.json`, the input they were compiled from.

Three gates per game, enforced by the forge:

1. `scene_lint` reports 0 findings (collision shapes, signal targets, resource ids, camera).
2. The Rust trace replay reaches the goal within 600 frames, using the same constants the GDScript exports.
3. `godot --headless --path depot/games/<slug> --quit-after 240` prints no ERROR/WARNING.

Plus a byte-identical re-export: the forge builds each game twice and refuses a
non-identical scene. **Never hand-edit `depot/`** — regenerate it.

**Win verification is a separate gate, and it is currently red.** `just verify-depot`
replays each game in Godot and requires the win line its runtime prints (`goal
reached`, `puzzle solved`, `door opened, win`, or `all hostiles down`). Today
**11 of 15** win: driving, flight, night-forest, platformer, runner,
snowy-mountain-pass, maze, pong, sokoban, chessboard-move, talk-inventory.

The four that do not are *unproven, not broken* — all boot with zero errors:

| Game | What is missing |
|---|---|
| `breakout` | The paddle tracks the ball, so every return is dead straight; the ball clears its own column and then loops. Bricks are destructible now, but 20 is unreachable. |
| `flappy` | Closed-loop altitude hold added; the bird still dies on pipes before x=55. |
| `topdown-shooter` | Bullet pool fixed (it was empty — the runtime filtered on invisibility), enemies die, but it stalls at 5 of 9. |
| `tower-place` | No placement policy; enemies never enter tower range. |

`just depot-godot` (boot clean) passing does not mean a game is winnable.

## Making a new game

The model writes a `GameSpec`, never GDScript. Rust validates and compiles it, so a
hallucinating model can produce a rejected spec but cannot produce a broken scene.

```powershell
cargo run -p gamecraft-cli -- new --prompt "a snowy mountain pass at dusk" --provider local
```

Provider notes, measured on this machine (see `docs/NEW_GAME.md`):

| Provider | Status |
|---|---|
| Ollama `gemma4:12b` (via `--provider local`) | works; needs `format: "json"`, `think: false`, `num_predict: 2500` |
| Ollama `qwen3.8:27b` | **crashes Ollama** (`wsarecv` reset, 0 bytes in 420 s) — do not use |
| OpenAI-compatible | wired (`max_tokens` + `response_format: json_object`); only exercised against a stub, no live key present |

## Monetisation (Phase 8)

`gamecraft-forge::economy` models monetisation as a deterministic state machine and
synthesises the store asset bundle. Both are library APIs today; exposing them as CLI
subcommands and MCP tools is the next step, not a claim of this release.

- `economy_lint(&EconomySpec) -> Vec<Finding>` — flags `ERR_FAUCET_DOMINANT` (faucet ≥
  sink means nothing is scarce), `ERR_NO_SCARCITY` / `ERR_SCARCITY_TOO_LATE` (target is
  day 4), `ERR_CURRENCY_NOT_DECOUPLED`, `ERR_PITY_INVERTED`, `ERR_PITY_TOO_LATE`
  (hard pity past 90 pulls), `ERR_NO_ANCHOR` (no $0.99 starter).
- `simulate(&EconomySpec, seed)` — seeded xorshift, day-by-day balances, pity-ramped
  pulls, longest gap between top-rarity hits. Reproducible: same seed, same numbers.
- `write_steam_bundle(slug, title, out)` — 20 files: 5 trading cards, 5 emoticons,
  3 profile backgrounds, 3 4K wallpapers, badge definition, Supporter Edition
  manifest, cloud-save JSON Schema, and a soundtrack.

**Honest scope note:** the soundtrack is `soundtrack.wav` (mono PCM, generated in
process). Real FLAC needs an encoder this crate does not carry, so the manifest names
the file that exists instead of promising one it does not.

Wallpapers are 3840×2160 and generated, not painted — they are placeholder-grade.
"Byte-complete" here means every file the store page references exists and is
well-formed, not that the art is good.

## Where to look

- `AGENTS.md` — repo layout, hard gates, rules.
- `CHANGELOG.md` — history, including the two serializer bugs found and fixed.
- `docs/NEW_GAME.md` — spec format, providers, measured model limits.
- `docs/MCP_CLIENTS.md` — wiring the MCP server into a client.
- `KIBITZ.md` / `KIBITZ_BACKTALK.md` — the review dialogue, kept as an audit trail.
