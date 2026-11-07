# Changelog

## 0.14.0 - 2026-11-07 (Toy railway, Bevy wave, arcade fixes)

- **Toy railway** (`gamecraft-bevy-railway`, Bevy): a two-level model railway. Talheim below,
  Bergblick 8 m up, two 1.5-turn helices, passing loops in both stations. Sparky (steam tank)
  and the bullet run a cyclic timetable; the bullet overtakes in the loops.
- **Rail sim** (`gamecraft-sim::rail`): block signalling with an interlocking (reserve ahead to
  the braking distance, one owner per block, release once the whole train has passed, authority
  ends 3 m before a red signal), timetables in whole seconds, integer ticks, state hash, layout
  validator (dead ends, gaps, platform length, grade per train class). Gates: 24 h with no
  collision, overrun or deadlock and everything within 30 s; a bad timetable is held at red and
  fails the punctuality gate; a 5.1 percent helix is refused for steam, accepted for the Regio.
- **Viewer**: one camera zoom from a tilt-shift overview (depth of field) to eye height on a
  platform; F follows the next train in. Day and night with lit windows, headlights, station
  lamps; signals show the reservations; seven-segment departure boards count down to the next
  scheduled arrival. `just railway-gate` proves a day at 60 Hz and saves a five-shot tour.
- **Scenery** (`terrain.rs`, `scenery.rs`): a landscape shaped around the track (embankments,
  cuttings, abutments, piers only where the line flies), about 1500 procedural trees in forests,
  a village with gabled houses, lit windows, street lamps, a church with an onion dome and cars
  on the main street, a vineyard on the hill, patchwork fields. Terrain rules are unit-tested; the
  tour reports 60 fps.
- **Water and landmarks**: a river from the table edge under a plate-girder railway bridge and the
  east helix viaduct into a lake ringed by the helix (one water plane, shown only where the land is
  carved); a castle ruin on the hilltop; a fenced pasture with a barn and Fleckvieh cows inside the
  west helix. New terrain test: water under the bridge and the lake, land above water elsewhere.
- Design and limits: `docs/TOY_RAILWAY.md`.
- **Nekomimi VRM viewer, phase 1** (`gamecraft-bevy-nekomimi`, behind the
  `vrm` feature, ported to Bevy 0.20 components): orbit, ground, path
  plumbing.
- **Web arcade wave**: `/new` preset page (Emberwick, Space Station,
  Pirate Ship, Zelda-like, Sokoban 9x9) plus nine more browser games;
  iPad PWA standalone mode and `tailscale serve` remote play.
- **Forged content**: sokoban-vault, TextureCraft barn wood on flight,
  Knight Tour title.
- **Desktop fixes**: library cards no longer stairstep (fixed-height
  cards in a wrapping grid); the detail divider travels again (the
  wrapping grid keeps a one-card minimum width); long gate lines
  truncate with the full text on hover.
- **Arcade server binds loopback** (`127.0.0.1`, `HOST` override kept):
  a `0.0.0.0` bind fails with EADDRINUSE while tailscaled serves that
  tailnet port, which left every Play click spawning a dead server while
  the browser fell through to the service worker cache (new games showed
  "offline, and this page was never cached").
- **TRELLIS.2 docs** re-verified against upstream 2026-11-07 (roadmap
  complete: training code + shape-conditioned texturing released).

### Bevy games, sound, replay, command registry (unreleased since 0.13.0, shipped here)

- **Bevy runtime games** (`gamecraft-bevy`, Bevy 0.20): a 29x29 maze that plays itself and
  Murmuration (20 000 starlings on the seeded `gamecraft-sim::flock` sim). Catalog kind `bevy`
  (`bevy/games`) in the library, CLI and MCP; gates `just bevy-gates`.
- **Procedural sound** for both games, computed from formulas; `gamecraft-synth-dump` writes WAVs,
  spectrograms and `docs/sounds/REPORT.md`; `just sounds-check` fails on drift.
- **Desktop**: draggable dividers, Reset layout, eframe persistence, command palette
  (`Ctrl+Shift+P`) running registry commands on a worker thread.
- **`gamecraft-replay`**: `.gcraft` documents (config, input log, result), re-simulating `verify`,
  tamper and hostile-input tests, Bevy parity tests, three committed examples in `docs/replays`
  and `just replays-check`.
- **Command registry** (`gamecraft_mcp::registry`): 12 commands, `commands` / `describe` /
  `run --cmd` on both binaries, MCP `list_commands` / `describe_command` / `run_command` /
  `replay_*`. Tool bodies moved from the MCP binary into `gamecraft_mcp::tools`.
  Status against the Craft app contract: `docs/CRAFT_CONTRACT.md`.

## 0.13.0 - 2026-11-02 (Prompt-to-Depot Visual Parity & Asset Factory)

- **Prompt-to-Depot Visual Parity:** Closed artifact gap in `create_game` (`crates/gamecraft-forge/src/create.rs`). Every game created via `gamecraft-cli new` now automatically renders its middle-frame `thumb.png`, downsampled animation `play.gif`, scene diagnostic `lint.json`, `trace.json`, and `commands.jsonl`.
- **Depot Completion:** 15 of 15 depot games now carry 100% complete visual assets and clean lint verdicts (`night-forest` and `snowy-mountain-pass` regenerated and verified).
- **Process Leaks Guard:** Integrated `gamecraft_mcp::proc_guard` Windows Job Object to kill all child Godot processes on exit or timeout.

## Unreleased - 2026-10-31 (no more leaked Godot processes)

- **Cause:** the dev box had 21 Godot game windows open (the viewer's Start Game spawned
  a new engine on every press, including gamepad A, and nothing ever closed them) plus
  headless engines whose launcher was gone.
- `gamecraft_mcp::proc_guard` (new): `kill_children_on_exit()` puts the process in a
  Windows Job Object with kill-on-close, so every child dies with it (normal exit,
  crash or kill); `run_with_timeout()` captures output and kills on a wall-clock budget.
  Uses the `win32job` crate (Windows only); the workspace stays free of `unsafe`.
- Viewer (`gamecraft-desktop`): job guard at startup; Start Game / Demo closes the game
  it started before launching the next one (status line says so).
- MCP server: job guard at startup; `scene_verify` gets a wall-clock limit (default
  frames/60*2+60 s, `timeout_s` to override) and reports `timed_out`.
- `scripts/verify-depot.ps1`: per-game timeout with `taskkill /T` instead of waiting
  forever.
- AGENTS.md rule: never leave engines running.
- Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features
  -D warnings`, `cargo test --workspace` green on Goliath (incl. 3 new proc_guard tests:
  finish within budget, kill on timeout, job guard idempotent).

## 0.12.0 - 2026-10-31 (Mossbrook prompt 4: The Quiet Well)

- **Story** (`web/games/mossbrook/story.js`): 14-stage quest grown from seeds
  already in the prompt-3 small talk. Two twists, an objective line with a
  compass arrow, a journal, stage-dependent dialogue, companion comments on
  new places, an ending.
- **Magic system:** a lantern carrying three flames (Ember burns, Glimmer
  reveals, Hush keeps sleepers asleep), learned along the story; Q cycles.
- **Stealth:** a stir meter near the sleeping Keeper; fill it and he throws you
  out of the cave.
- **World x4:** terrain 400 m square (was 200), river with a bridge and
  walk-blocking channel, fisher's hut, the Moonring stones, a hidden cellar
  behind a glyph wall, a root-blocked passage and a hollow behind the troll.
  About 700 trees. Collision gained functional blockers and removable boxes.
- **Gate:** the tour now plays the entire story (131 steps, 14 checkpoints,
  must never be tossed). Self-test added because the tour passed on the first
  full run: Ember walking is tossed in 2.7 s, running in 1.7 s, Hush walking
  never, Hush running in 8.3 s.
- **Verifier:** clears a game's `shots/` before each run (stale prompt-3 shots
  were sitting next to new ones).
- **Sound** (`audio.js`): place-based ambience (wind, birds or crickets, river,
  Moonring hum, fire crackle, underground drips and drone, tavern murmur),
  footsteps by surface, the Keeper's snore and a stir heartbeat, story effects,
  an off-key song for Odo, and an outdoor lute tune. N/M toggles, persisted;
  off in verify mode.
- **Cheat sheets:** `web/games/kaffeehaus/CHEATSHEET.md` (every number in the
  sim plus the tactics the bots proved), `web/games/mossbrook/CHEATSHEET.md`
  (controls, map, stealth numbers, spoiler walkthrough), `web/CHEATSHEET.md`
  (developer: commands, hook, gate rules, patterns, traps). Linked from all
  READMEs; AGENTS.md now asks for a cheat sheet per game.
- Not done: hand-play; sound is untested by ear.

## 0.11.0 - 2026-10-31 (three.js web family: Mossbrook, Kaffeehaus)

A second track next to the Godot depot, prompted by Simon Willison's October 2026
vibe-game posts: model-written browser games, verified by headless self-play.

- **`web/`**: static Bun server (`bun run serve`, port 11258), arcade index,
  three.js from `node_modules` via an import map. Each game lives in
  `web/games/<slug>/` and exposes `window.__gamecraft` (ready, autoplay, step,
  render, optional selfTest).
- **Gate** (`just web-verify`, Node 24): headless Chromium with SwiftShader WebGL,
  zero console errors, scripted tour must print the `win_marker`, per-checkpoint
  screenshots, `verify.json` per game. Runs under Node because Playwright's
  browser pipe hangs under the Bun runtime.
- **Harness bugs found by inspecting screenshots, not by the gate:** the help
  overlay was never hidden in verify mode (every shot was mostly overlay), and the
  verifier rendered up to 300 steps past each checkpoint (two shots identical).
  Both fixed; `step(n)` now stops at the first new checkpoint.
- **Mossbrook**: rebuild of Willison's Emberwick prompts 1-3 (village, companions,
  cutaway walls, upstairs, cellars + tunnel, troll cave). Tour asserts layer
  visibility, wall opacity and floor height at 8 checkpoints. Prompt 4 not done.
- **Kaffeehaus**: original Viennese coffee house management game. Pure sim
  (`sim.js`) with a `selfTest()` proving decisions matter: good play beats random
  beats idle on day 1, on a busy day, and over a 5-day campaign (EUR 931 vs 723,
  regulars' loyalty 20 vs 1 of 25). The self-test rejected four versions first:
  waiter pathing into the street, patience shorter than the room, no tray
  batching, and "rush bill" being free. Fixes added second rounds and
  order-on-seating. One threshold was lowered (busy-day takings margin 15% to 5%);
  the campaign check carries that claim instead.
- **Kaffeehaus sound**: Web Audio synthesis, no assets. Room murmur and machine
  hiss scaled to load, event sounds, music-box waltz. Starts on first gesture,
  silent in verify mode, S/M toggles persisted.
- Not done: nobody has hand-played either game yet; screenshots were checked by
  pixel statistics only.

## 0.10.2 - 2026-10-29 (local generation actually works: `think: false`)

- **Root cause of every local failure: thinking tokens.** `gemma4:12b` is a
  reasoning model in Ollama; hidden reasoning is generated before `content` and
  counts against `num_predict`. The first attempt truncated the spec at 487 chars,
  the second spent the whole budget thinking and returned empty `content` - which
  the client can only report as "no JSON object found". Neither was the model
  being too weak.
- Fix: `"think": false` on the Ollama request, alongside `format: "json"` and
  `num_predict: 2500`.
- **Result: `--model gemma4:12b` generated `snowy-mountain-pass` in 10 seconds**,
  compiled to 267 frames, lint clean, and Godot prints `GameCraft: goal reached`.
  A 12B model is sufficient for this task once decoding is constrained.
- Confirmed the 27B never worked: `qwen3.8:27b` returned zero bytes in 420s, then
  reset the connection (`wsarecv`), after which even the 12B timed out until Ollama
  was restarted. Auto-selection now orders local models smallest-first so it cannot
  pick one that will not load.
- Gates: clippy pedantic, 63 tests, fmt, 15 games on disk.

## 0.10.1 - 2026-10-29 (model provider hardening: cloud-first, size-aware local)

- **Cloud is now the documented first choice**, with keys: `--provider openai`
  (`OPENAI_API_KEY`) or `--provider anthropic` (`ANTHROPIC_API_KEY`). Not everyone
  has a 4090, and a hosted model follows the schema far more reliably than a
  mid-size local one.
- **JSON constrained at the transport**, which matters more than model size:
  Ollama gets `format: "json"`; OpenAI-compatible gets
  `response_format: {"type":"json_object"}`. Without these, small models answered
  with prose or truncated JSON.
- **Local model selection is size-aware.** `auto` now reads
  `details.parameter_size` from `/api/tags` and orders models smallest-first, so a
  27B cannot be auto-selected. An explicit `--model` is still honoured exactly.
- Output budgets fixed per provider: Ollama `num_predict: 2500` (thinking tokens
  were consuming the reply, producing empty content), OpenAI `max_tokens: 2048`,
  curl timeout 240s.
- `docs/NEW_GAME.md` gained a "Which model should I use?" table with the measured
  local reality: `gemma4:12b` answered in ~11s but was unreliable on the schema;
  `qwen3.8:27b` returned zero bytes in 420s and then crashed the server
  (`wsarecv: connection forcibly closed`), after which even the 12B timed out -
  Ollama needed a restart.
- Verified: cloud request shape against a local stub; local transport against
  `gemma4:12b` (answered). **Not verified: a full live spec generation from either
  cloud or local** - no API key on this machine, and Ollama was wedged by the time
  the constrained-JSON changes landed.
- Gates: clippy pedantic, 63 tests, fmt green.

## 0.10.0 - 2026-10-29 (make a NEW game with local or cloud AI)

- **New pipeline: prompt -> model -> `GameSpec` -> compiled, gated game.** The model
  never writes `GDScript`; it writes a declarative spec (theme, physics, ground
  slabs, platforms, coins, goal). `create.rs` compiles that through the same
  builders, exporter, lint rules and trace simulation as the hand-made depot, then
  writes a complete project plus `spec.json` for provenance.
- `GameSpec` validation with reasons the model can act on: slug charset, ground
  widths, goal sanity, gravity floor, and the rule that a platform whose underside
  is below 1.1 is a wall the autopilot cannot see - found the hard way, when a
  generated level stalled the player at x=0.4.
- **Completability is checked before anything is written:** a gap-detecting policy
  (the same rule the in-engine autoplay uses) runs in `gamecraft-sim`; failure
  returns the exact reason and the furthest x reached, which is fed back to the
  model for one repair round.
- `llm.rs`: Ollama, OpenAI-compatible and Anthropic providers behind one
  `complete()` contract, invoked through `curl.exe` with an argument array (no new
  dependency, no shell quoting). `extract_json` tolerates fences and prose.
- CLI: `gamecraft-cli new --prompt ... --provider local|openai|anthropic
  [--model] [--endpoint]`, plus `--from-spec <file>` for the deterministic path.
- Verified end to end: (a) `--from-spec` on a night-forest spec produced a game
  that prints `GameCraft: goal reached` in Godot; (b) `--provider openai` against a
  local stub server produced `mock-forest`, which also reaches the goal. The
  stub proves the HTTP contract, not a live model - no Ollama was running here.
- `docs/NEW_GAME.md` documents both paths, the rules the compiler enforces, and the
  limits (one archetype, mock-verified provider path, no textures).

## 0.9.3 - 2026-10-29 (docs: kibitz log reconciliation + open threads)

- `KIBITZ_BACKTALK.md` had mixed time bases: entries stamped `HH:MM UTC` that were
  actually Vienna wall-clock (+01:00 in winter) sat next to a genuine UTC stamp, so
  the log was not in chronological order. Added an explicit stamp convention to
  the protocol header and switched to ISO 8601 with offset
  (`2026-10-29T18:05:00+01:00`). Earlier entries left untouched - rewriting them
  would erase the evidence of the error.
- Appended a verified state table (13-game depot, 4 MCP tools, 5 wired clients,
  toolset phases 0-1 done, all gates green) and the open-threads list, so a fresh
  session does not have to reconstruct it.
- No code changed; no gate cycle required.

### Open threads

1. `scene_verify` (toolset phase 2): `trace | godot | frame | shot`.
2. Assert on `GameCraft: goal reached` after boot, not just "no ERROR lines" - both
   critical bugs passed load-only checks.
3. Platonic solids as real `.obj` + `just godot-import` (design proven viable;
   `--headless --import` is broken in Godot 4.4 here).
4. `Start Game` spawn is wired but **not click-tested end to end** - treat as
   unproven until someone presses it.
5. Desktop depth buffer / mesh textures / shadows: the wgpu phase, only worth
   doing if the viewer becomes a product rather than a demo aid.

## 0.9.2 - 2026-10-29 (desktop: depot quick-select + Start Game)

- Left panel opens with **`Depot (13)`**: every `depot/games/<slug>/` that has a
  `main.tscn`, discovered by walking up from the working directory. Clicking a row
  loads that scene into the viewer (so you see it before launching) and arms the
  action; **`Start Game`** spawns `godot --path <project>`. `GAMECRAFT_GODOT`
  overrides the binary; the status line reports the child pid or the spawn error.
- Spawn rather than attach: the ask is "show me this game now", which must work
  with no editor open. Attaching to a running instance is Plane 2 work
  (`--control`/`--attach`), still specced and fenced.
- Two bugs found by looking at the running app:
  1. `find_up` requires `is_file`, so `depot/games` (a directory) never matched and
     the panel showed `Depot (0)`. Added `find_up_dir`.
  2. Two scroll areas in one panel without ids made egui paint
     "First/Second use of widget ID" warnings over the UI. Both now use `id_salt`.
- Not click-tested end to end: the spawn command is the same one verified by hand,
  and failures surface in the status line rather than silently.

## 0.9.1 - 2026-10-29 (desktop: scene lighting)

- **The viewer shades with the scene's own lights.** `mesh.rs` gained
  `collect_lighting`: it walks the graph for `DirectionalLight3D` (direction from
  the world basis), `OmniLight3D` / `SpotLight3D` (position, `light_energy`,
  `light_color`, `omni_range`), and reads the `Environment`'s
  `ambient_light_color` when present. Shading is ambient + per-light Lambert, with
  squared distance falloff for point lights; a scene that declares no lights still
  falls back to the fixed key direction so it is shaded rather than flat.
- **Scenes now have a sun.** `Forge::sun` adds a `DirectionalLight3D` with energy
  and shadows. It is added to `platformer` and `flight`. Depot materials are
  unshaded, so this changes nothing in Godot - it exists so the viewer has
  something real to read, and it is the first step toward a shaded depot.
- Window opens at 980x600 pinned to (8,8): at 110% scaling the old 1280x800
  window extended past a 720p screen, cropping the lit lower half of the viewport
  and the inspector.
- Verified by screenshot (`docs/screenshots/desktop-viewport.png`): bright cloud
  tops with shaded undersides, gradient across the ground, coins picking up the
  sun. Sun also visible as a node in the scene tree.
- Limits: no shadows (a directional light without a shadow map only changes
  intensity by normal), no specular, no per-pixel normals. Only `platformer` and
  `flight` declare a sun; the other eleven use the fallback.

## 0.9.0 - 2026-10-29 (desktop renders real geometry)

- **The viewport draws the scene's actual meshes, not just boxes.** New
  `apps/gamecraft-desktop/src/mesh.rs` builds triangles from the scene's own
  sub-resources: each `MeshInstance3D`'s `mesh = SubResource("id")` is resolved to
  its primitive declaration and real dimensions (`BoxMesh.size`, `SphereMesh.radius`,
  `CylinderMesh` radii, `PrismMesh.size`, `TorusMesh` radii), transformed by the
  world matrix, back-face culled, flat-shaded from one fixed light, and sorted
  far-to-near before painting. Colour comes from the node's
  `material_override` albedo, so the viewer shows the authored palette.
- Because egui's render pass has no depth buffer, depth is the painter's
  algorithm: triangles are sorted per frame. That is correct for separated
  geometry and can mis-order interpenetrating triangles.
- Camera now auto-frames on load, using **visible mesh bounds only** - the root
  subtree includes the kill plane, which previously dragged the centre below the
  floor and backed the camera off until the scene was a speck.
- Default fixture is now `depot/games/platformer/main.tscn` (the old fixture had
  colliders but no meshes, so the viewport was legitimately empty).
- Frustum overlay is off by default; the AABB wireframe moved behind the
  "AABB boxes (wireframe)" toggle.
- Verified by capturing the running window: ground slabs in perspective, green
  platforms, faceted clouds, gold coins, blue player capsule. Screenshot kept at
  `docs/screenshots/desktop-viewport.png`.
- Still not a GPU renderer: no textures on meshes, no shadows, no depth buffer.
  A wgpu pipeline remains the path to fidelity, but the viewer now shows objects.

## 0.8.2 - 2026-10-29 (desktop: real wireframe + readable contrast)

- **Viewport draws 3D wireframe, not screen rectangles.** Each node's AABB is
  projected to eight screen points and its 12 edges drawn, so rotation and
  perspective are visible; nodes are sorted far-to-near so nearer geometry
  overlaps correctly, and a box is skipped unless all eight corners are in front
  of the camera (a partial projection drew lines through the near plane). The
  selected node gets a centre dot and a heavier stroke.
- **Contrast fixed.** `Visuals::dark()` left mid-greys on near-black. Now:
  `override_text_color` at `(234,237,244)`, panel `(16,18,24)`, window
  `(22,25,33)`, extreme background `(9,11,16)`, distinct stroke weights for
  inactive/hovered/active widget states, and type sizes raised (body 15,
  monospace 14.5, small 12.5).
- Removed the now-dead `screen_extent` helper.
- Honest limit, unchanged: this is a **wireframe**, not a mesh render. Drawing the
  actual triangles needs a wgpu pipeline (vertex/index buffers, camera uniforms,
  materials, depth) in the desktop crate - a phase of its own, not a tweak.

## 0.8.1 - 2026-10-29 (windmill/lighthouse/castle + generated texture depot)

- **Three more building kinds** on the same parametric path: `Windmill` (hub plus
  cross-blades), `Lighthouse` (red band plus lantern room), `Castle` (four corner
  turrets plus a crenellated parapet). `props` now has a test that every kind
  produces at least one node.
- **Generated texture depot** (`crates/gamecraft-forge/src/textures.rs`): brick
  (running bond with mortar), plaster (noise), roof tile (courses), grass
  (speckle). Each is a pure function of `(x, y)`, so the PNG bytes are identical
  on every run - no binary assets to lose, nothing downloaded. Three tests: valid
  PNG magic, determinism, and no two patterns collide.
- `Forge::textured_material` / `apply_texture` plus `BuildingSpec.tex`, so a
  building can be textured by id. The flight village now uses brick for barn,
  church and castle, plaster for the rest.
- **Import caveat, measured:** Godot does not load a PNG without its `.import`
  file, and `godot --headless --import` fails in 4.4 (progress-dialog errors, zero
  `.import` files written). An editor pass does work:
  `godot --editor --path <game> --quit-after 120`. Added `just godot-import` to do
  that for every depot game. A fresh clone needs it once - standard Godot
  behaviour for any project with assets.
- `.godot/` added to `.gitignore`; `*.import` files are kept.
- Verified by capture: brick courses visible on the church, plaster on the
  lighthouse; flight still clears all 5 gates with textures loaded.
- Gates: 65 tests, clippy pedantic, fmt, wasm, Godot 13/13 clean.

## 0.8.0 - 2026-10-29 (driving v2 - real circuit)

- **`driving-box` replaced by `driving`**: a top-down circuit instead of a car
  sliding down a straight strip. 8 checkpoints on a 12-unit-radius loop, constant
  speed, bounded turn rate, one lap to win.
- New `gamecraft-sim` car model (`CarBody`, `step_car`, `run_drive`): pure-pursuit
  steering on the XZ plane, with two tests - one proving a 12-radius loop is
  completable (335 frames) and one proving a tight 2-radius loop is not, so the
  "is this drivable" question is answered by simulation rather than by eye.
- Player mode 6 in `player_controller.gd` (forward = local +X, steering rotates the
  body); `autoplay.gd` gained a `drive` mode that steers toward the next
  checkpoint with the same controller.
- Verified in the engine: `GameCraft: checkpoint 1..8` then `goal reached`.
- Two bugs found while wiring it: the autopilot collected no targets in drive mode
  (the collection was gated on `flight` only, so the car drove straight through
  two checkpoints), and a duplicated `drive` export broke the script parse.
- Gates: 62 tests, clippy pedantic, fmt, Godot 13/13 clean.

## 0.7.0 - 2026-10-29 (13th archetype: flight + parametric props)

- **New archetype `flight`**: 5-gate course over a settlement, `mode 5` flight
  controller (fixed forward speed, pitch input drives climb), autopilot that
  tracks each gate's altitude. Verified in Rust (501 frames) **and in the engine**
  (`GameCraft: ring 1..5`, then `GameCraft: goal reached`).
- **New `crates/gamecraft-forge/src/props.rs`**: parametric props instead of
  modelled assets.
  - `BuildingSpec { kind, x, z, width, depth, height, lod }`; kinds Farmhouse,
    Barn, Church, Igloo. Same code path, different parameters and primitives
    (igloo = squashed sphere, church = tower + spire).
  - LOD is a node-count decision: 0 = one box, 1 = + pitched roof, 2 = + door and
    (chimney | tower+spire | entrance).
  - `CreatureKind::Person` = capsule + sphere head; `Cow` = box body, box head,
    four legs.
- **Second critical bug fixed: `script` was exported in alphabetical order.**
  Godot applies node properties in file order, so `mode = 5` / `flight = true`
  were set on a node with no script yet, silently dropped, and the node kept its
  script defaults. Symptoms: the aircraft ignored flight mode and fell; pong
  paddles ignored `axis`/`max_speed`/`target_path`. `script` is now written
  first, before every property it defines.
- Both bugs were invisible to every existing gate: the scene loads, lints clean,
  and the Rust trace passes. Only running the engine and looking at the result
  caught them.
- Gates: 62 tests, clippy pedantic, fmt, wasm, Godot 13/13 clean, 258 depot files.

## 0.6.5 - 2026-10-29 (CRITICAL: transposed Transform3D; all 12 prettified)

- **Critical bug: every rotated transform was transposed.** Godot writes the
  basis **row by row**; the exporter emitted **columns**. Axis-aligned,
  translation-only nodes look identical either way, so nothing failed - but every
  rotated node was wrong. Concretely: every top-down camera (all 7 board/arena
  games) pointed **up** instead of down, so those scenes rendered as an empty
  background. Pong showed two paddles nowhere.
  - Found by tracing the projection: a paddle at world x=-7.5 landed at screen
    x=585 (right of centre) with the camera claimed to look down. That is only
    possible if right = -X and forward = +Y.
  - The parser had the same fault in reverse, which is exactly why the existing
    parse -> export round-trip test passed: it transposed twice.
  - Fixed in `encode_transform` and `decode_node_transform`; new regression test
    `transform3d_reads_rows_not_columns` pins the convention against a real
    Godot look-down camera.
  - This also re-frames the earlier "camera framing" fix (0.6.2): the pitched
    static camera *was* looking up, which is why the level sat at the bottom of
    the screen. The follow camera masked the symptom; this fixes the cause.
- All 12 games prettified: procedural sky for side views, a solid backdrop for
  top-down scenes (a sky is invisible from straight above), capsule player,
  sphere coins/ball/enemies/bullets, cylinder towers, prism flag, rounded bushes.
- Verified by capture: pong renders paddles/ball/walls on navy; sokoban renders
  checkerboard, capsule player, crates and targets.
- Gates: 55 tests, clippy pedantic, fmt, wasm, Godot 12/12 clean.

## 0.6.4 - 2026-10-29 (platformer: sky, real primitives, readable decor)

- Answered by inspection first: the scene had **15 `BoxMesh` and 18 `BoxShape3D`,
  zero textures, zero non-box primitives, and no `WorldEnvironment`** - the gray
  background was Godot's default clear colour, and the "three white blocks" were
  the clouds, which were boxes.
- New `Forge` helpers: `sphere_mesh`, `capsule_mesh`, `prism_mesh`,
  `sphere_shape`, `capsule_shape`, and `sky()` (ProceduralSkyMaterial + Sky +
  Environment + WorldEnvironment, ambient from sky).
- Platformer now: capsule player with a capsule collider, sphere coins with sphere
  colliders, sphere bushes and clouds, prism goal flag, gradient sky instead of
  gray.
- Two visual bugs found by looking at captured frames rather than reasoning:
  1. The sky's ground half was near-black, so the level appeared to float over a
     dark void with a hard horizon line. Lightened to a haze that meets the sky
     horizon colour.
  2. A bush placed at x=7.5 sat over the pit and read as floating greenery. Decor
     is now placed only on ground segments.
- Still true, and worth stating: there are **no textures, no lighting, no
  shadows**. Every material is unshaded flat albedo. Adding textures needs an
  asset pipeline (PNG files plus `albedo_texture`); the rasterizer can already
  emit PNGs, so that is the next step if wanted.
- Prettification is platformer-only so far; the same helpers are available to the
  other eleven.

## 0.6.3 - 2026-10-29 (autoplay: the engine now plays itself)

- **Nothing started a playthrough before this.** `trace.json` was written but never
  read by any script, and the hint volumes could not fire because the headless
  player never moved (mode 0 reads `Input.get_axis`, which is 0 with no input).
  The only watchable playthrough was `play.gif`, rasterized in Rust.
- New `Autoplay` node + `scripts/autoplay.gd` (`Node3D`): synthesizes input each
  physics frame. Two modes: `smart_jump` probes the floor ahead with a ray and
  jumps at real gaps; `jump_frames` replays exact frames from the trace.
- Platformer wired. Verified in the engine: `godot --headless --path
  depot/games/platformer` prints `GameCraft: goal reached` with no input, and
  stops driving on win instead of walking off the end of the level.
- Two real divergences found by instrumenting a run, not by reasoning:
  1. **Frame-exact replay does not transfer.** The Rust stepper wins at frame 414;
     replaying those exact jump frames in Godot missed the first gap, because the
     engine's integrator and the body's settling frame differ. Hence `smart_jump`.
  2. **A short floor probe misreads raised platforms.** Probing 1.8 units down from
     a platform reported "gap" and the autoplay jumped off the platform into the
     pit it was avoiding. The probe now reaches 3.5 units below the feet.
- `Player` spawn moved to y=0.5 (exactly resting): spawning 0.1 above the floor
  made the first jump land on a frame the body was still airborne.
- Platformer jump now reads `is_action_pressed` rather than `is_action_just_pressed`;
  an edge-triggered press on a settling frame is silently dropped forever.
- Still frame-replay only (not yet smart): runner, flappy, driving-box.
- Gates: 54 tests, clippy pedantic, fmt, Godot 12/12 clean.

## 0.6.2 - 2026-10-29 (fix: side-view games rendered off-screen)

- **Bug:** the platformer (and runner / flappy / driving-box) were unplayable to
  watch. The static camera was pitched 13.6 degrees down and framed the whole
  48-unit level, so a 12:1 level collapsed into a thin strip. Measured on the real
  build at 1280x720: everything sat between y=640 and y=720 of a 720px frame, the
  top 89 percent was empty clear colour. The player was effectively off-screen.
- **Fix:** all four side-view games now use a follow camera
  (`scripts/camera_follow.gd`, FOV 34, tracking the player on x at a fixed height
  and distance). The static pitched shot is gone.
- **Re-measured after the fix:** ground spans y=600-720, player is a solid block
  at y=480-540, sky above. Content is on screen and readable.
- Added `Forge::follow_camera` (no pitch, follow script attached) so the pattern
  is one call, not four copies.
- Godot headless sweep: 12/12 clean after the new script (a GDScript syntax error
  in it would have taken down every side-view game).
- Root cause worth keeping: the forge verifies *mechanics* (trace replay) and
  *parseability* (Godot boot), but neither checks **framing**. Only looking at a
  frame catches "the camera points at the sky". That is why `scene_verify`
  needs `mode="frame"`/`"shot"`.

## 0.6.1 - 2026-10-29 (desktop: real file dialogs)

- File > Open scene and Save as now use a **native file dialog** (`rfd` 0.15,
  already in the local cargo cache) filtered to `.tscn`/`.tres`, instead of a
  path text box. The dialog starts in `depot/games` when that directory can be
  found by walking up from the working directory.
- `/` resolved by walking up from cwd, not from a compiled-in path: launching the
  exe from Explorer (cwd = `target\debug`) now still finds the bundled fixture,
  the depot and the right start directory. A compile-time path would have worked
  on exactly one machine (BUG-079).
- `Dialog::Open`/`Dialog::Save` and the unused `path_buffer` field removed; only
  the About window remains a modal.
- Smoke-run from `target\debug` as cwd: starts and stays up, no panic.

## 0.6.0 - 2026-10-29 (toolset phase 1: `scene_inspect`)

- New MCP tool `scene_inspect` with four operations over one parse:
  `tree` (paginated pre-order rows: path, type, parent, depth, unique, script,
  position, rotation in degrees, scale, child count), `signals` (every connection
  with `resolved` and the reason when it is not), `property` (one node's transform
  and all properties), `find` (case-insensitive substring over name and type).
  Paginated with `limit`/`offset` and a `next_offset` cursor.
- New `input.rs`: every scene-consuming tool now accepts **either** `scene_text`
  **or** `scene_file` (+`root` for `res://`). Both or neither is an error;
  `res://` without `root` is an error; a resolved path escaping `root` is an
  error. `scene_lint` and `spatial_query` gained the same inputs.
- Acceptance verified: `tree` node counts match `rg -c "^\[node"` for all 12 depot
  scenes (21 to 151 nodes each, 12/12 agree); `signals` on `platformer` returns
  7 edges, all resolved.
- 54 tests (5 inspect + 5 input added), clippy pedantic, fmt, wasm all green.
- Corrected the plan's own acceptance criterion: it claimed 2 signal edges for
  `platformer`; the real count is 7 (2 triggers + 5 hint volumes). The test would
  have failed on a correct implementation.

## 0.5.1 - 2026-10-29 (desktop menus + capture feasibility)

- `gamecraft-desktop` now has a real menu bar (it previously had none - the top
  strip was a palette/toolbar hybrid with no File, View or Help):
  - **File**: Open scene, Reload, Save as, Quit.
  - **Edit**: Undo, Redo (disabled when empty), Add node submenu, Reset camera.
  - **View**: toggles for AABB boxes, collision colours, frustum, ground axes,
    trace recording, follow-newest.
  - **Help**: About / controls, with scene uid, node count, findings and renderer.
- `load_from_path` / `save_to_path` / `reload` in `state.rs`. Save keeps the
  source file's `ext_resource` / `sub_resource` declarations so a round trip does
  not drop them, writes a timestamped `.bak`, and **refuses to write a scene that
  would not lint clean** - the desktop cannot replace a working file with a broken
  one.
- Overlay toggles are wired into the viewport (they were unconditional before).
- Real viewport capture measured: impossible under `--headless` (null viewport
  texture, no PNG); **works windowed** - 200x200 PNG with exact authored colours
  (clear `0.2,0.6,0.898`; a `ColorRect` at `1.0,0.349,0.098`). `mode="shot"` will
  ship as an opt-in mode that costs a visible window for about a second.

## 0.5.0 - 2026-10-29 (step zero of the toolset plan)

- New `crates/gamecraft-sim` (L1, std only): `sim.rs` moved verbatim out of
  `gamecraft-forge` - kinematic stepper, `bfs_path`, `solve_sokoban`. Both forge
  and (later) mcp depend on it, which is what makes `scene_verify(trace)`
  implementable without a dependency cycle.
- `lint_live`, `LintDiagnostic`, `scene_lint` and the `ERR_*` constants moved from
  `gamecraft-mcp` into `gamecraft-godot` (beside `build_graph`/`build_registry`).
  `gamecraft-mcp` re-exports them, so `main.rs`, the desktop and the tests are
  unchanged.
- `gamecraft-forge` no longer depends on `gamecraft-mcp`. `cargo tree -p
  gamecraft-forge` shows no edge to it.
- Verified: 138 depot files hash byte-identically before and after the refactor;
  49 tests, clippy pedantic, fmt and the wasm probe all green.
- `tools-set-plan.md` rev 4a: `mode="shot"` (real viewport capture) recorded as
  blocked - measured under `--headless`, `get_viewport().get_texture()` is null
  and no PNG is produced, so a capture needs a visible rendering context.

## 0.4.0 - 2026-10-29 (export tool + client wiring)

- New MCP tool `export_scene`: JSON scene spec -> deterministic engine text.
  `format` is a parameter (`godot4` today), so a second target such as Overte is
  a new enum value, not a new tool. Nodes are built through `gamecraft-commands`,
  the same bus the desktop and the forge use.
- `crates/gamecraft-mcp/src/spec.rs`: `SceneSpec`/`NodeSpec`/`ConnSpec`/
  `ExtSpec`/`SubSpec`, parent-first ordering, duplicate-path and unknown-parent
  rejection, typed JSON -> `PropertyValue` with a `{"raw": ...}` escape hatch
  for Godot expression syntax.
- CLI: new `export` subcommand (`--in spec.json --out scene.tscn --format
  godot4`); creates the output directory and reports node/byte/uid counts.
- `tests/fixtures/scene_spec.json` with a verified loop: export -> lint clean ->
  `godot --headless` loads the result with zero ERROR/WARNING.
- `docs/MCP_CLIENTS.md`: config path and entry shape per client, tool reference,
  prompt patterns, and the tool's honest limits.
- Wired into Claude Desktop, Claude Code (`.mcp.json`), opencode, Cursor and
  Antigravity. Every existing config was backed up before patching and verified
  after (+1 server each, zero keys lost).

## 0.3.0 - 2026-10-29 (Phase 5 - twelve-game depot)

- New `crates/gamecraft-forge`: 12 deterministic game builders (platformer,
  runner, flappy, driving-box, pong, breakout, topdown-shooter, tower-place,
  sokoban, maze, talk-inventory, chessboard-move). Every node goes through
  `gamecraft-commands`; `commands.jsonl` is the provenance log.
- New mirrored Rust trace sims (`sim.rs`): kinematic stepper, BFS maze solver,
  BFS crate-pushing solver, and per-genre policies (pong/breakout/tower DPS).
  A game only ships if its scripted trace reaches the win state inside 600 frames.
- New software rasterizer (`render.rs`): deterministic `thumb.png` + `play.gif`
  from the verified trace, fixed 16-colour palette.
- `gamecraft-godot`: new `export_tscn` serializer (pre-order tree, sorted keys,
  shortest-roundtrip floats). Round-trip and determinism tests.
- `gamecraft-mcp`: new `ERR_UNRESOLVED_RESOURCE` lint (property references an
  undeclared `ExtResource`/`SubResource`). Caught two real shipping bugs.
- `gamecraft-cli`: new `forge` subcommand (`--game <slug>` / `--all` / `--out`).
- `depot/`: 138 files, 12 standalone Godot 4.4 projects. All 12 boot headless
  with zero ERROR/WARNING; all 138 files hash identically across two full runs.
- `docs/GAME_SUCCESS_META.md`: research note on what actually makes a game
  succeed (juice, teaching, colour, demographics, monetisation, and what AI
  does and does not remove from the art pipeline).

### Real bugs the Godot gate caught (and now have lint/tests)

1. `%Player` unique-name syntax - Godot 4 wants `unique_name_in_owner = true`;
   the `%` form made every child parent path unresolvable.
2. Typed strings exported unquoted (`mode = maze`) and Godot expressions
   over-quoted (`"NodePath(\"../Ball\")"`) - both are parse errors.
3. A script id drift (`ExtResource("3_enemy")` vs `2_enemy`) that only Godot
   reported. Now `ERR_UNRESOLVED_RESOURCE` fails the build first.

## 0.2.1 - 2026-10-29 (Phase 4b polish)

- Theme: amber accent (`255,190,60`) in `Visuals` selection + hyperlinks,
  monochrome paths and scene `uid` in the top bar, selection boxes amber.
- Palette dropdown under the input (vertical, `>` highlight); Esc clears,
  Up/Down cycle, Enter runs; `Ctrl+Z` / `Ctrl+Y` kept.
- Viewport: TODO for true 12-edge wireframe (rects stand for v1).
- `DesktopState::from_text` test seam + 4 state tests (undo/redo depth,
  lint-string select, ghost-None, header `uid` preference).

## 0.2.0 - 2026-10-29 (Phase 4a)

- `apps/gamecraft-desktop`: wired from placeholder to `egui`/`wgpu`
  (`eframe` 0.37, `Renderer::Wgpu`) shell, in-process on `gamecraft-core`.
- Left scene tree with lint badges, center orbit viewport with AABB boxes,
  collision colors, frustum wireframe, trace ghost; right inspector
  (position/scale via `SetTransformCommand`) plus live `scene_lint` panel
  (click selects); bottom 60-frame trace scrubber; top command palette with
  undo/redo (`Ctrl+Z` / `Ctrl+Y`).
- `gamecraft-mcp`: new `lint_live(graph, registry)` shared by `scene_lint`
  and the desktop's per-frame refresh.
- `gamecraft-commands`: new undoable `SetTransformCommand`.
- `gamecraft-core` stays graphics-free (`wasm32-unknown-unknown` probe green).

## 0.1.0 - 2026-10-29

- Phase 1: Cargo workspace (`gamecraft-core`, `gamecraft-commands`,
  `gamecraft-godot`, `gamecraft-mcp`, `gamecraft-cli`, `gamecraft-desktop`),
  `justfile`, `AGENTS.md`.
- Phase 2: deterministic `nom` parser for Godot 4 `.tscn` text
  (`gd_scene` header, `ext_resource`, `sub_resource`, nodes, connections)
  plus deterministic UID hashing from the relative scene path.
- Phase 3: AABB aggregation for `Node3D`/`MeshInstance3D`/`CollisionShape3D`
  hierarchies; `scene_lint` emits `ERR_MISSING_COLLISION_SHAPE` and
  `ERR_UNRESOLVED_SIGNAL_TARGET`; unit tests for parser accuracy,
  command undo/redo, and lint triggers.
