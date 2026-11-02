# New targets: plan

Written 2026-11-01 (Claude Opus, session with Sandra). Status lines are updated in place as work lands.

GameCraft has two runtimes today: forge-compiled Godot 4 projects (`depot/games`) and
hand-built three.js pages (`web/games`). This plan adds targets in order of leverage.

## The bar a target has to clear

A target is a first-class citizen only if it fits the existing pattern:

1. **Text out of a spec.** The game (or scene) is produced deterministically from data,
   so the same input gives the same bytes and the output can be diffed and gated.
2. **A headless gate.** Something proves the result works without a person: a win
   marker, checkpoints, a validator. No gate, no catalog entry.
3. **One catalog.** `gamecraft_mcp::catalog` lists it, so `game_list`, `gamecraft-cli
   list` and the desktop Library show it without special cases.

Targets that cannot meet (2) are allowed as showpieces, labelled as such.

## Order

| # | Target | Kind | Gate | Effort | Status |
|---|---|---|---|---|---|
| 1 | glTF 2.0 export | Neutral scene format | Khronos glTF validator + three.js load | 1-2 days | **done** (`just gltf-gate`) |
| 2 | PWA wrapper for the three.js games | Packaging | `just web-pwa`: manifests, icons, precache, every game boots offline | 0.5 day | **done** |
| 3 | Bevy runtime | Third game runtime | `cargo test` steps the game headless to a win | 4-6 days | planned |
| 4 | Resonite live build | Social VR world | Sim-side gate only; world itself checked by hand | 3-4 days | planned (experiment) |
| 5 | Fantasy console (TIC-80) | Tiny cartridges | Spike first: is there a usable headless mode? | 1 day spike | spike |
| 6 | Boomy (physical robot) | Showpiece | Sim-side gate only | 2 days | idea |

Effort is AI-assisted working days, not calendar weeks.

## 1. glTF 2.0 export

**Why first.** Cheapest, and it multiplies everything else: one exporter feeds Blender,
three.js, Godot, Unity, Resonite, Bevy (which loads glTF natively) and any viewer.
`export_scene` already takes `format` as a parameter for exactly this.

**Design.**

- New L1 crate `crates/gamecraft-gltf` (no graphics or UI deps; glam + serde_json only).
- `geometry` module: the primitive tessellation (box, sphere, capsule, cylinder, prism,
  torus) moves here from `apps/gamecraft-desktop/src/mesh.rs`. The desktop viewport and
  the exporter then build identical triangles from one source.
- `export` module: `SceneGraph` + the scene's sub-resources -> glTF 2.0.
  - One glTF node per scene node, local TRS, same hierarchy and names.
  - `MeshInstance3D` with `mesh = SubResource(...)` -> a mesh (flat normals, POSITION +
    NORMAL accessors with min/max).
  - `material_override` albedo -> a PBR material; Godot's sRGB colour converted to the
    linear `baseColorFactor` glTF expects. Materials are de-duplicated.
  - `DirectionalLight3D` / `OmniLight3D` -> `KHR_lights_punctual`, `Camera3D` -> camera.
  - Two encodings: `.glb` (binary, for files) and `.gltf` JSON with an embedded base64
    buffer (text, for MCP results).
  - Deterministic: no timestamps, stable ordering, same scene -> same bytes.
- Surfaces:
  - MCP: `export_scene` gains `format: "gltf"` (embedded JSON in the result).
  - CLI: `gamecraft-cli gltf <scene.tscn> [--out file.glb]` converts any existing Godot
    scene, so every depot game is exportable today.
  - Desktop: File > Export glTF (later in this phase).

**Gate.** Unit tests (GLB header and chunk alignment, accessor counts, determinism,
winding), then `just gltf-gate`: every depot scene is exported, run through the Khronos
`gltf-validator` (zero errors), and rendered in three.js in headless Chromium
(`web/scripts/gltf-shot.ts`, PNGs in `target/gltf/shots`).

**Decisions made while building.**

- Meshes are shared by geometry (type + properties), not by sub-resource id: the forge
  declares one `BoxMesh` per wall, so the maze went from 354 meshes / 510 KB to 4 / 53 KB.
- Leaf `CollisionShape3D` nodes are dropped (physics, not geometry); named marker nodes
  (spawns, triggers) are kept because importers (Bevy, Resonite) need them. The validator
  reports those as `NODE_EMPTY` infos, which is expected.
- Exports are faithful: the shooter's bullet pool parked at y = -50 is in the file.
- Fixed on the way, in the shared geometry (so the desktop viewport changed too): torus
  normals pointed inward on the inner surface, capsules had no bottom cap, `PrismMesh`
  ignored `left_to_right` (Godot default is a centred apex), and capsule/cylinder used
  wrong defaults (Godot 4: radius 0.5, height 2).

**Out of scope.** Textures, skinning, animation, collision shapes (they are not visual).

## 2. PWA wrapper for the three.js games

Each game installs as its own app and plays offline; the arcade installs as "GameCraft".
Built as: `web/pwa.ts` (manifests from `game.json`, a generated network-first service
worker that precaches all 127 URLs, head tags injected by `serve.ts` at serve time, so game
folders carry no boilerplate), `web/pwa.js` (registration, skipped under `?verify`),
`scripts/pwa-icons.ts` (square icons cropped from `thumb.png`, arcade mosaic).

Gate `just web-pwa` (`scripts/pwa-check.ts`): every manifest has the required fields and
192/512 px PNG icons of the declared size; the worker takes control and precaches every
URL; then with the network cut, the arcade and all 13 games boot from the cache with no
page errors or failed requests. Phones need HTTPS: `tailscale serve --bg 11258` (tailnet
only), documented in `web/README.md`.

## 3. Bevy runtime

**Why.** The only candidate that adds a third *verified* runtime and shares code instead
of translating to it: `gamecraft-core`, `gamecraft-sim` and the solvers are Rust already.

**Shape.** `gamecraft-forge` gains a Bevy backend for the archetypes that have a sim
(maze, sokoban, runner, platformer first). Output: a small Cargo project under
`depot/bevy/<slug>/` that depends on a shared `gamecraft-bevy` runtime crate. Level geometry
comes in as glTF from step 1. The gate is a headless test (`MinimalPlugins`, fixed timestep,
built-in autoplay) that must reach the win state within a frame budget. WASM build for the
browser comes free.

**Fleet standard.** `mcp-central-docs/standards/rust-crates/BEVY.md` governs this step:
Bevy **0.20** (verified there 2026-10-17), `default-features = false` with only the
features used, `bevy/dynamic_linking` while iterating, Bevy isolated in its own workspace
member (never a leaf crate), headless runs via `ScheduleRunnerPlugin`. It also requires a
written justification, so here it is:

> **Why Bevy, not `three-d` or egui.** This is a game runtime, not a viewer: it needs an
> ECS game loop with a fixed timestep, input, cameras, collision and per-entity behaviour,
> driven by the same specs the Godot forge compiles. `three-d` renders scenes but has no
> game loop or ECS; egui is UI. Bevy is also the only candidate where the existing Rust sims
> (`gamecraft-sim`) run in-process as systems, so a game's win condition can be asserted in
> a plain `cargo test`. The build cost is contained by keeping Bevy in `gamecraft-bevy`
> only; `cargo check -p gamecraft-core` and the MCP/CLI crates never pull it in.

**Risk.** Bevy breaks its API every minor release; the standard budgets half a day per
bump. Pin one version in the workspace and update deliberately; generated projects never
pin independently. Clean builds approach 10 minutes, so the gate builds once and runs all
games from one binary.

## 4. Resonite live build

Use resonite-mcp (ResoniteLink protocol) to build a world in a running session from the
same scene data, instead of writing files. Spot the Difference and Where is Benny are the
natural first worlds: they are better as rooms people walk through together. Gate: the
puzzle logic keeps its sim-side gate; the built world is checked by hand and the catalog
marks it "manual check". glTF from step 1 is the asset path for anything non-primitive.

## 5. Fantasy console spike (TIC-80)

Cartridges are one small text file with hard limits, which keeps spec-made games small.
TIC-80 is FOSS; PICO-8 is commercial. Before any forge work, a one-day spike answers: can a
cartridge run headless (CLI mode, no window) long enough to detect a win marker? If not, it
stays a showpiece.

## 6. Boomy

Robot Sorting already has a rules engine and sim. A physical version drives Boomy via
yahboom-mcp to sort real objects. Sim-side gate only; a demo, not a catalog runtime.

## Log

- 2026-11-01: plan written; step 1 started (crate scaffold, geometry move, exporter).
- 2026-11-01: step 1 done. `gamecraft-gltf` crate; CLI `gltf <scene>` and
  `gltf --depot-out <dir>`; MCP `export_scene format=gltf`; desktop File > Export glTF;
  `just gltf-gate` green on all 15 depot games (0 errors, 0 warnings). Next: step 2 (PWA).
- 2026-11-01: step 2 done. `just web-pwa` green (arcade + 13 games installable, 127/127
  precached, all boot offline); gameplay gate `bun run verify` re-run green with the
  injected head tags. Next: step 3 (Bevy), plan aligned with mcd BEVY.md (0.20).
- 2026-11-01: step 3 first slice done. gamecraft-bevy (Bevy 0.20.1, default features off, render behind a feature) plays the maze on the shared sim. The depot 15x15 maze was too easy (32 moves, 7 dead ends, 6 junctions), so the Bevy default is now the 29x29 seed 146 challenge maze: 272 moves, 21 dead ends, 19 junctions, 391 open cells, all at least 3x, enforced by a test. The depot Godot maze is unchanged (it stays byte-gated, and MazeParams caps size at 25), so the twins now differ on purpose. just bevy-gate green: autoplay wins 272/272 moves, 0 bumps, screenshot in docs/bevy-maze.png. Next: Sokoban on Bevy, then a catalog entry once a second archetype exists.
- 2026-11-02: Murmuration, the second Bevy game. The flock sim lives in `gamecraft-sim::flock` (std only, seeded, bit-identical across runs and thread counts): 20 000 boids with a counting-sort grid, a neighbour cap, pillars, three hawks that strike for several boids at once, a player lure, a flare ability (6 s cooldown) that scatters hawks, and a roost the flock must settle in (85 percent alive and inside). `autopilot` is the scripted player the gate uses. Balance, measured: the autopilot wins every seed losing 1 to 3 percent of the flock; a careless player (lure straight at the roost, no flares) loses 6 to 15 percent and fails outright on at least one seed (locked in by a test). First version had harmless hawks (slower than a panicking flock) and a 30 m roost that could not hold 16 000 boids; both were found by measuring and fixed. A second pass fixed a thin comet-shaped flock (the lure pull and alignment squashed it) so it now splits around pillars and re-forms; spread measures 15 to 110 m. Performance: steering runs on up to 8 threads (`std::thread::scope`, fixed chunks, no result change) because the single-threaded step cost about 15 ms once the cloud spread, which dropped the window to 10 fps; now 60 to 67 fps at 20 000 boids. Bevy side: `MurmurationLogicPlugin` (resources only, headless tests), `render_murmuration` (one entity per boid, three shared unlit materials, follow camera with C to toggle the whole arena, live HUD in the window title), binary `gamecraft-bevy-murmuration` (arrows or WASD steer, Space flares, `--autoplay`, `--exit-on-win`, `--screenshot F --shot-tick T`, `--overview`, `--flock N`). `just murmuration-gate` is green: autoplay settles the flock at tick 2564 (71 eaten), identical to the headless sim, and a mid-flight screenshot (`docs/murmuration.png`) must contain a visible flock. Build notes: new Cargo profile `play` (own code opt 1, dependencies opt 3, `gamecraft-sim` opt 3); its first build is long (about 15 minutes here) and hit a Windows paging-file error (1455) when run with full parallelism, so use `-j 6`. Not done at that point: mouse steering (needs a cursor-ray, not just the keyboard feature), sound, bloom, a catalog entry. Sound and the catalog entry followed the same day (next entry).
- 2026-11-02 (later): sound, a Bevy catalog kind, and draggable panels. Sound: `gamecraft-bevy::synth` computes every effect from formulas (hand-rolled xorshift noise, one-pole filters, sweeps, a cross-faded 8 s ambient bed) and wraps it as 16-bit PCM WAV; `cues` turns game state into cues (hawk cry rate-limited to one per 20 ticks, flare when the sim's `flare_age` drops, win or loss once, maze step, bump and win); `audio` (feature `audio`, now part of `render`) plays them. Bevy features added on purpose: `bevy_audio` and `wav` only; this pulled rodio 0.22 and cpal 0.17 into the lock file. Unit tests cover the waveforms (audible, below clipping, bit-identical, ambient loop seam no louder than the bed's own steps, WAV header) and the cue rules, all without a sound card. Runs with `--exit-on-win` (the gates), `--mute` or `GAMECRAFT_MUTE=1` stay silent. I cannot hear it, so `gamecraft-synth-dump` (`just sounds`) writes each effect as a WAV, a spectrogram PNG and a loudness table into `docs/sounds/`, committed on purpose and guarded by `just sounds-check`. Reading the spectrograms caught a real defect at once: the ambient bed was broadband hiss (centroid 2.4 kHz) instead of low wind, fixed with a three-pole filter (centroid 130 Hz). Taste (is the cry a hawk?) still needs a listen to docs/sounds; the levels (ambient 0.3, cries 0.5, flare 0.7) are set from the measured RMS and peaks. Catalog: new `GameKind::Bevy` (entries in `bevy/games/<slug>/` with `game.json`: title, category, experiment, bin, profile, win_marker; `CHEATSHEET.md`, `thumb.png`, `verify.json`). `GameKind::ALL/label/groups/parse` replaced the per-site runtime lists. `bevy_verify` builds the binary, lets it play itself and writes `verify.json`; `just bevy-gate` and `just murmuration-gate` record their pass with `scripts/bevy-record.ps1`. MCP `game_verify` for Bevy is opt-in (`GAMECRAFT_MCP_ALLOW_BEVY=1`), the CLI `verify` runs it, the desktop Play and Watch demo start the built binary or print the exact build command if it is missing (no cargo build inside the UI). Desktop: left and right panels have explicit size ranges, the depot list now fills the left panel instead of capping at half its height, and a draggable divider separates it from the scene tree; the inspector and findings are split the same way. View > Reset layout restores defaults. Sizes are not persisted across restarts (eframe persistence is not enabled in this build). A kittest test drags the divider and checks the scene pane grows and that Reset restores it.
- 2026-11-02 (replay and registry): `gamecraft-replay` (L1) defines the `.gcraft` document: game, config, input log keyed by tick, recorded result. `verify` re-simulates from seed and inputs and compares status, ticks and an FNV-1a state hash (tamper tests: edited input, swapped seed, forged hash, hostile ranges). The maze rules exist twice, as a pure function in the replay crate and inside the Bevy schedule; `crates/gamecraft-bevy/tests/replay_parity.rs` feeds a replay into the real Bevy systems and requires the same result (maze moves, bumps and win tick; murmuration full-state hash, bit-identical). The command registry (`gamecraft_mcp::registry`) lists 7 commands (`game.list`, `game.report`, `scene.lint`, `scene.inspect`, `replay.record`, `replay.verify`, `replay.info`) with menu path, shortcut and typed params; CLI (`commands`, `describe`, `run --cmd`) and MCP (`list_commands`, `describe_command`, `run_command`, `replay_*`) both call `registry::run`, and the shared tool bodies moved from the MCP binary into `gamecraft_mcp::tools`. Three example replays are committed in `docs/replays` (depot maze, challenge maze, a 2000 starling flock that settles in 2749 ticks) with `just replays-check`. Honest scope: the registry covers 7 of the app's capabilities, the desktop app does not use it yet, and the live plane, release factory and golden PNGs are not started; see `docs/CRAFT_CONTRACT.md`. Float determinism across machines is still untested.
- 2026-11-02 (registry, round two): the remaining MCP tool bodies (`spatial_query`, `export_scene`, `game_verify`, `game_create`, `scene_verify`) moved into `gamecraft_mcp::tools` and are registered, so the registry has 12 commands. The desktop app got a command palette (`Ctrl+Shift+P`, Commands menu): it lists the registry, prefills required params, runs the command in-process on a worker thread and shows the JSON result; a command's declared shortcut opens it on that command. Tests: GUI output equals `registry::run` for success, failure, bad JSON and unknown ids; a kittest drives record and verify through the window (it also caught two text fields without accessible names, now labelled). Still direct calls, not through the registry: the older menu items and library buttons. Not registered: `forge`, `new`, `gltf`.
