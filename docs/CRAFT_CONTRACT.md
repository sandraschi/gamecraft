# GameCraft against the Craft app contract

Status of GameCraft against the Craft app standards in `mcp-central-docs`
(`standards/RUST_ARTCRAFT_STANDARD.md`, `standards/rust-artcraft/CRAFT_COMMAND_BUS.md`,
`CONTAINMENT_AND_CRATES.md`). Written to be honest about gaps: "done" means it runs and is
tested, "partial" says what is missing, "not started" means nothing exists.

Nothing here has been reviewed or accepted by the Craft fleet. As far as I could find there is
no published acceptance process for community Craft apps; this is a self-assessment.

## What this step added

| Piece | Where | State |
|---|---|---|
| Command registry (`commands --json`, `describe <id>`, `run --cmd 'id={json}'`) | `crates/gamecraft-mcp/src/registry.rs` | done for 12 commands, see below |
| Command palette in the desktop app (runs registry commands in-process on a worker thread; `Ctrl+Shift+P`) | `apps/gamecraft-desktop/src/palette.rs` | done, with a GUI-equals-registry test |
| `.gcraft` replay document (seed + config + input log + result) | `crates/gamecraft-replay` | done for maze and murmuration |
| Replay verify (re-simulate, compare status, ticks, state hash) | `gamecraft_replay::Replay::verify` | done, tamper tests |
| Bevy parity (real schedule fed from a replay lands on the same state) | `crates/gamecraft-bevy/tests/replay_parity.rs` | done, bit-identical hash |
| Committed example replays and a gate | `docs/replays/`, `just replays-check` | done |
| MCP tools `replay_record`, `replay_verify`, `replay_info`, `list_commands`, `describe_command`, `run_command` | `crates/gamecraft-mcp/src/main.rs` | done |

## Registered commands

| Id | Menu | Shortcut | Read-only |
|---|---|---|---|
| `game.list` | Library/List Games | | yes |
| `game.report` | Library/Game Report | | yes |
| `scene.lint` | Scene/Lint | Ctrl+L | yes |
| `scene.inspect` | Scene/Inspect | Ctrl+I | yes |
| `game.verify` | Library/Verify Game | | no (runs the gate, writes `verify.json`) |
| `game.create` | Library/Create Game | | no (writes a game folder) |
| `scene.spatial_query` | Scene/Bounding Box | | yes |
| `scene.export` | Scene/Export | | yes |
| `scene.verify` | Scene/Verify in Godot | | no (launches Godot; needs `GAMECRAFT_MCP_ALLOW_GODOT=1`) |
| `replay.record` | Replay/Record | Ctrl+R | no (writes the `.gcraft` you name) |
| `replay.verify` | Replay/Verify | | yes |
| `replay.info` | Replay/Info | | yes |

Ids are `<area>.<verb>` and never renamed. The MCP tool name is the id with `.` replaced by `_`
(`replay.verify` is `replay_verify`), per the tool naming trap in the contract. In the desktop app a
command's shortcut opens the command palette with that command selected and its required
params prefilled; it does not run on the keypress, because most commands need parameters.

## Contract checklist

| Contract item | State | Detail |
|---|---|---|
| L1 / L2 / L3 seam | partial | L1: `gamecraft-core`, `-commands`, `-sim`, `-replay`, `-godot`, `-forge`, `-gltf`. L2: `gamecraft-mcp` (library plus binary). L3: `gamecraft-desktop` (egui) and `gamecraft-cli`. Bevy lives in `gamecraft-bevy` only. The registry sits in the `gamecraft-mcp` library because the catalog code it wraps lives there; a purist split would move catalog and tools into an L1 crate. |
| Lint walls | done | Workspace denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, all of `clippy::pedantic`, and forbids `unsafe`. Tests opt out of `unwrap` locally. |
| Hostile input fence | partial | Replay and registry inputs are range-checked (see below). The older tools (`scene_*`, `game_*`) were not re-audited in this step. |
| CLI first, `mcp` subcommand in one binary | partial | `gamecraft-mcp` now takes `commands`, `describe`, `run` and `mcp`; no arguments still means stdio MCP so existing client configs keep working. The older subcommands (`lint`, `list`, `verify`, `forge`, `new`, `gltf`, `export`) are still only in `gamecraft-cli`. |
| Every CLI command has `--json` | partial | The registry subcommands do. The older `gamecraft-cli` subcommands: `list`, `report`, `verify` do, others print JSON or text as they always did. |
| Universal command bus, `dispatch(cmd)` never panics | partial | `registry::run(id, params)` never panics and returns a `ToolError` (code, suggestion, remediation) instead. It is a table of functions keyed by id, not the contract's `enum Command` with typed variants. The existing `gamecraft-commands` crate is the scene undo stack, a different thing. |
| CLI and MCP share one code path | done | Both call `registry::run`. The tool bodies moved from the MCP binary into `gamecraft_mcp::tools` for this. A test checks `replay_info` and `run_command` give identical answers. |
| Commands cover the whole app | partial | All nine original MCP tools are registered now, plus the replay commands. Not registered: `forge`, `new` (LLM front end), `gltf` (CLI only), and every desktop-only action (open scene, save as, layout reset, depot load, play). The registry is not yet "the whole API". |
| In-process parity: GUI calls the same dispatch | partial | The command palette calls `registry::run` in-process and a test (`gui_execute_equals_registry_run`) requires its output to equal the registry's for success, failure, bad params and unknown ids; another drives record and verify through the palette window with `egui_kittest`. The older menu items and library buttons still call library functions directly and are not covered. |
| Live plane: `--control PORT`, `mcp --attach PORT` | not started | Needs a loopback server in the desktop app executing registry commands on the UI thread. No port is allocated for GameCraft in the fleet table (`WEBAPP_PORTS.md`, 14100 to 14112); that allocation is the fleet owner's call. |
| `egui_kittest` golden PNGs | not started | One kittest interaction test exists (scene divider drag and reset). No golden images. |
| `--smoke` headless launch | not checked | Not looked for in the desktop app in this step. |
| Release factory (local cut, NSIS installer, GitHub release) | not started | Existing release notes are manual. |
| Oracle loop | not applicable | GameCraft re-implements no original app, so there is no oracle to hold a parity floor against. The replay check plays that part for game rules: a rules change that moves a committed replay's hash fails `replays-check`. |
| Binary size under 2 MB (tier 1) | not measured | The Bevy games are far larger; the MCP and CLI binaries were not measured. |
| Definition of done: clippy, tests, fmt, MCP stdio probe, lockfile, changelog | see below | |

## Replay hostile-input rules

A `.gcraft` file is untrusted. Before any simulation runs:

- format must be `gcraft`, version must be 1;
- file size at most 64 MiB; declared ticks at most 1,000,000;
- maze size odd and within 5..=151; input ticks start at 1, never exceed the declared ticks and
  strictly increase;
- flock count 1..=100,000, at most 64 hawks and 64 pillars; order coordinates finite and within
  one million; order ticks strictly increase;
- the registry only reads or writes paths ending in `.gcraft`, and `replay.record` refuses to
  overwrite unless `overwrite` is true.

Invalid maze steps (not exactly one axis by one) are ignored, exactly as the Bevy schedule ignores
them, so a hostile log cannot make the two disagree.

## What a replay proves, and what it does not

`verify` re-simulates from the seed and inputs and compares status, tick count and a FNV-1a 64
fingerprint of the final state (every boid position and velocity for murmuration). It catches
edited inputs, a swapped seed, a forged result and rules that changed since recording. It is not
cryptographic: someone who understands the format can write a different input log and a matching
hash, so it is a consistency check, not anti-cheat.

Floating-point determinism is proven on one machine (same binary, any thread count; Bevy schedule
and pure replay agree bit for bit). Across different CPUs or compilers it is untested, and flock
steps use `f32` math. Before this is used as a multiplayer lockstep format, run the committed
replays on a second machine; `just replays-check` is that test.

## Format (`.gcraft` version 1)

Compact JSON, one line:

```
{"format":"gcraft","version":1,"engine":"gamecraft-replay 0.10.2",
 "game":"maze","config":{"size":15,"seed":9},
 "inputs":[{"tick":1,"dx":0,"dy":1}, ...],
 "result":{"status":"won","ticks":187,"state_hash":"39e877fb3b17140e","summary":{...}}}
```

Murmuration uses `"orders":[{"tick":0,"x":..,"y":..,"flare":false}, ...]`: a change list. The lure
heads for `(x, y)` from `tick` until the next entry; `flare` fires on its own step only.
`engine` is informational and never checked.

## Deferred, in the order they would pay off

1. Move the older desktop menu items and library buttons onto `registry::run`, register `gltf`,
   `forge` and `new`, and add the desktop-only actions. Turns the registry into the whole API.
2. Live plane (`--control PORT`, `mcp --attach PORT`) once a port is allocated.
3. Desktop: golden PNGs for the palette and library, a `--smoke` mode.
4. Release factory: NSIS installer and the triple-play release.
5. Engine crate (UCI / USI / GTP) with a vendored chess front end; engines stay separate
   processes, never linked.
