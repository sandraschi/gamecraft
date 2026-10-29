# GameCraft

**Describe a game in a sentence. Get a Godot 4 project you can open, play, and edit.**

```powershell
gamecraft-cli new --prompt "a snowy mountain pass at dusk, hard jumps" --provider local
```

That runs a language model (a local Ollama model by default) which writes a small
JSON description of a game: sky colours, jump strength, where the platforms and
coins go, how far the goal is. GameCraft validates that description, compiles it
into a real Godot scene, boots Godot headless, and checks the game can actually be
finished. If it can't, you get a failure, not a broken level.

The output is a normal Godot project folder. Open `project.godot` in the editor,
press F5, or hit Start Game in the desktop viewer.

## What you get

`new` writes a minimal but complete Godot project:

```
depot/games/snowy-mountain-pass/
  project.godot      # open this in Godot 4.4
  main.tscn          # the generated scene
  spec.json          # the model's description, the input to the compiler
  scripts/           # player, camera, runtime
  icon.svg
```

The hand-built depot games additionally carry `lint.json`, `trace.json`,
`commands.jsonl`, `thumb.png`, `play.gif` and a `README.md`. The prompt path does
**not** produce those yet - no thumbnail, no recorded replay, no per-game lint
dump. That gap is real and not hidden here.

## What the AI actually controls

| Dial | Effect |
|---|---|
| `theme` | six colours: sky, horizon, ground, platforms, player, coins |
| `physics` | run speed, jump impulse, gravity - this is the "game feel" |
| `level` | ground slabs, floating platforms, coin positions, goal distance |
| `title` / `slug` | name and folder |

A full example is committed at `depot/games/snowy-mountain-pass/spec.json`.

## What it cannot do yet

Straight answers, because this is the part that matters:

- **Three archetypes, not thirteen.** Prompts can produce `platformer`, `runner`
  and `pong`. Ask for a shooter and the compiler rejects the spec and lists what is
  supported - it does not silently hand back a platformer.
- **What each archetype lets you change:** `platformer` gets the full level (ground
  slabs, platforms, coins, goal) plus physics and theme; `runner` gets theme and
  name (its jump timing is a fixed verified script); `pong` gets theme, name and
  ball speed (8-22).
- **The other ten genres exist but are hand-built.** The 15 demo games span
  platformer, runner, flight, driving, pong, breakout, flappy, maze, sokoban,
  chessboard, dialogue, shooter and tower defence. Only the three above are wired to
  the prompt path. Flappy stays unwired on purpose: it is one of the four that never
  proved an in-engine win, so generating it would ship games I cannot verify.
- **Local model quality varies.** `gemma4:12b` works. `qwen3.8:27b` crashes Ollama
  outright (socket reset, no output in 420 s) - do not use it.
- **Cloud providers are wired but only tested against a stub.** No live API key
  was available, so OpenAI/Anthropic paths are unproven end to end.

## Why "spatial compiler"

The name describes the plumbing, not the pitch.

A language model asked to "write a game" will invent function names that do not
exist and produce GDScript that fails on line 3. So the model here never writes
code. It writes data. GameCraft then:

1. **Validates** the data (supported archetype, positive sizes, reachable goal).
2. **Compiles** it deterministically into Godot scene text - same spec, same bytes,
   every run.
3. **Verifies** the result: lint the scene, replay the level in Rust, then boot
   Godot and require the win line to print.

A hallucinating model can therefore produce a *rejected spec*. It cannot produce a
broken scene, because the compiler is the only thing that writes scenes.

## Is the output actually any good?

Checked, not assumed. `just verify-depot` boots every depot game in Godot and
requires its win line:

```
11 of 15 games win
4 unproven: breakout, flappy, topdown-shooter, tower-place
```

All 15 boot with zero engine errors. The four that do not win are documented with
their specific cause in the Depot section - they are broken designs, not unverified
guesses, and the gate stays red to say so.

## The 15 demo games

`depot/games/` is the proof, not the product - examples of what the compiler emits,
each with a trace recording and a playable Godot project. The interesting use is
generating your own.

## Quickstart

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# gates
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo check -p gamecraft-core --target wasm32-unknown-unknown

# make a game from a prompt (local Ollama by default)
cargo run -p gamecraft-cli -- new --prompt "night forest, hard jumps"

# or skip the model entirely and compile a spec you wrote
cargo run -p gamecraft-cli -- new --from-spec depot/games/snowy-mountain-pass/spec.json

# check every depot game still boots and wins
just depot-godot
just verify-depot

# viewer
cargo run -p gamecraft-desktop

# MCP server over stdio
cargo run -p gamecraft-mcp
```

`new` needs neither Godot nor a GPU: it compiles and lints the scene in Rust and
replays the level in the Rust sim. You need **Godot 4.4** on `PATH` (or
`GAMECRAFT_GODOT`) to play the result or to run the depot gates, and **Ollama** with
a model pulled for `--provider local`. `--provider auto` probes local servers first.
`--from-spec` skips the model entirely.

## Controls

**In the games**: arrow keys or WASD to move, Space/Enter to jump.

**Gamepad.** Godot's built-in `ui_*` actions carry joypad bindings for movement
(D-pad and left stick) but `ui_accept` ships keys-only, so a pad could walk and
never jump. The shared runtime adds the missing buttons at startup:

| Input | Effect |
|---|---|
| D-pad / left stick | move, walk, drive, chess cursor |
| **A** | jump, flap, confirm knight move |
| **B** | quit the game (there is no menu to return to) |

**Desktop viewer** (`gilrs`, polled per frame): left stick orbits the camera,
D-pad or stick vertical moves the depot selection, **A** starts the highlighted
game, **B** returns selection to the scene root. Keyboard and mouse still work; a
missing pad degrades to "not connected".

## Depot

`depot/games/` holds 15 standalone Godot 4.4 projects. Each carries `project.godot`,
`main.tscn`, `scripts/`, `lint.json`, `trace.json`, `commands.jsonl`, `thumb.png`,
`play.gif` and a `README.md`; the two produced from a spec also carry `spec.json`.

Three gates per game, enforced by the forge:

1. `scene_lint` reports 0 findings (collision shapes, signal targets, resource ids, camera).
2. The Rust trace replay reaches the goal within 600 frames, using the same constants the GDScript exports.
3. `godot --headless --path depot/games/<slug> --quit-after 240` prints no ERROR/WARNING.

Plus a byte-identical re-export: the forge builds each game twice and refuses a
non-identical scene. **Never hand-edit `depot/`** - regenerate it with `just forge`.

### Win verification, and why it is red

Booting is not winning. `just verify-depot` replays each game in Godot and requires
the win line its runtime prints (`goal reached`, `puzzle solved`, `door opened, win`,
or `all hostiles down`).

11 of 15 win today: driving, flight, night-forest, platformer, runner,
snowy-mountain-pass, maze, pong, sokoban, chessboard-move, talk-inventory.

The four that do not are unproven, not broken - all boot with zero errors:

| Game | What is missing |
|---|---|
| `breakout` | The paddle tracks the ball, so every return is dead straight; the ball clears its own column then loops. Bricks are destructible, but 20 is unreachable. |
| `flappy` | Closed-loop altitude hold added; the bird still dies on pipes before x=55. |
| `topdown-shooter` | Bullet pool fixed (it was empty - the runtime filtered on invisibility), enemies die, but it stalls at 5 of 9. |
| `tower-place` | No placement policy; enemies never enter tower range. |

## Building blocks

| Crate | Role |
|---|---|
| `gamecraft-core` | Node graph, transforms, AABB, collision matrix. No graphics deps; compiles to `wasm32-unknown-unknown`. |
| `gamecraft-commands` | Undoable command stack for every scene mutation. |
| `gamecraft-godot` | Deterministic `.tscn`/`.tres` parser **and serializer**, UID hashing, lint rules. |
| `gamecraft-sim` | Deterministic gameplay sims: kinematic stepper, BFS maze and crate solvers. |
| `gamecraft-forge` | Game builders, `GameSpec` validation and compilation, the LLM front end, thumbnail rasterizer, economy and Steam asset generators. |
| `gamecraft-mcp` | MCP stdio server: `scene_lint`, `spatial_query`, `export_scene`, `scene_inspect`. |
| `gamecraft-cli` | `lint`, `query`, `uid`, `forge`, `new`, `export`. |
| `gamecraft-desktop` | `egui`/`wgpu` viewer: depot browser with Start Game, shaded mesh viewport, inspector, trace scrubber, undo/redo. |

Games are **compiled from a spec, never hand-written in GDScript** - that is what
makes the byte-identical depot gate possible.

## Monetisation (Phase 8)

`gamecraft-forge::economy` models monetisation as a deterministic state machine and
synthesises a store asset bundle. Both are library APIs today; exposing them as CLI
subcommands and MCP tools is the next step, not a claim of this release.

- `economy_lint(&EconomySpec)` flags faucet-dominant economies, missing or late
  scarcity (target is day 4), undecoupled currency tiers, inverted or late pity
  (past 90 pulls), and a missing $0.99 starter anchor.
- `simulate(&EconomySpec, seed)` - seeded, reproducible day-by-day balances with
  pity-ramped pulls.
- `write_steam_bundle(slug, title, out)` - 20 files: trading cards, emoticons,
  profile backgrounds, 4K wallpapers, badge definition, Supporter Edition manifest,
  cloud-save JSON Schema, soundtrack.

**Honest scope note:** the soundtrack is `soundtrack.wav` (mono PCM generated in
process). Real FLAC needs an encoder this crate does not carry, so the manifest names
the file that exists. Wallpapers are generated gradients - placeholder-grade.

## Where to look

- `AGENTS.md` - repo layout, hard gates, rules.
- `CHANGELOG.md` - history, including the two serializer bugs found and fixed.
- `docs/NEW_GAME.md` - spec format, providers, measured model limits.
- `docs/MCP_CLIENTS.md` - wiring the MCP server into a client.
- `KIBITZ.md` / `KIBITZ_BACKTALK.md` - the review dialogue, kept as an audit trail.
