# GameCraft - Independent Assessment (Claude)

- **Date:** 2026-10-29, about 13:45 CET (repo state as of that moment; the builder was still active)
- **Author tag:** Claude (Sonnet 5.5), via Claude Desktop on Goliath
- **Target:** `D:\Dev\repos\gamecraft` (local only, no git repo yet)
- **Scope:** read-only on the repo. This file is the only thing I wrote there. All experiments ran on a copy in `d:\dev\repos\temp\gc-depot-run`.
- **Companion doc:** `ASSESSMENT-antigravity-2026-10-29.md` (written ~12:05, before the forge and depot existed). Cross-checked in section 7.

---

## 1. Verdict

The engineering core is good: a disciplined, lint-clean, WASM-safe Rust workspace with a real scene parser, a deterministic exporter, a working MCP server, an egui viewer, and a forge that emits 12 complete Godot projects. Gates I ran myself are green.

The Phase 5 claim ("12/12 verified, zero manual fixups") is **not yet true**. I found a systematic exporter bug that makes real Godot silently discard most per-game configuration (`mode`, `gravity`, `jump_velocity`, `autofire`, `dialogue`, grid sizes and more) on every one of the 12 games. Rust-side lint and trace verification cannot see it, because the verification replays a Rust re-implementation of the physics with the intended parameters, and the Godot boot test only checks that nothing prints an error. I reproduced the failure in real Godot 4.4 and proved the root cause by fixing it in a scratch copy (runner then reaches "goal reached" at about frame 400; the Rust trace predicted 415).

The fix is small (hours). The larger lesson is that the verification loop needs one more rung: execute the game in Godot and assert the win, not just boot it. Hold the Storytold and HN steps until that exists.

---

## 2. What I checked and how

| Check | Result |
|---|---|
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --all --check` | clean |
| `cargo check -p gamecraft-core --target wasm32-unknown-unknown` | green |
| `cargo test --workspace` | 45 pass, 0 fail (commands 6, core 12, desktop 4, forge 4, godot 9+2, mcp 8). Builder's log says 41; it has grown since. |
| Godot 4.4 headless boot of all 12 depot games (copy) | 0 errors, 0 warnings. Matches the builder's claim. |
| Godot 4.4 headless run, `--fixed-fps 60`, 2400 frames | 11 of 12 print nothing; pong runs but has no win condition (71 points, no end). |
| Probe script on runner and platformer (player position per second) | Player never moves in either game. |
| Same probe after moving `script =` to the first line of the Player node (scratch copy only) | Runner auto-runs, hint jumps fire, `GameCraft: goal reached` at about frame 400. |

Reading coverage, to be honest about it: I read forge `lib.rs` and `sim.rs` in full, the shared GDScript runtime and player controller, all PLAN files, README/AGENTS/CHANGELOG/justfile, KIBITZ tails, and the depot artifacts. I skimmed or did not read: forge `arcade.rs`, `combat.rs`, `puzzle.rs` (about 100 KB of scene builders), the desktop app beyond its tests, `getsometaste.md`, and most of `GAME_SUCCESS_META.md`. Findings about those areas are limited to what the build, tests and outputs show.

---

## 3. What is solid

- **Layering is real.** `gamecraft-core` has no graphics or Godot deps and compiles to `wasm32-unknown-unknown`. Forbidden-unsafe and denied `unwrap`/`expect`/`panic`/`todo`/`pedantic` hold across about 9,000 lines of Rust with clippy clean. That is an unusual bar and it is actually enforced.
- **Determinism is genuine.** The forge builds each game twice and compares byte for byte, then writes. Property keys are sorted and floats formatted stably. This is also the root of the bug below, but the principle is right.
- **The command log is a good primitive.** Every scene mutation is an undoable `Command` serialized to `commands.jsonl`. It powers the desktop undo stack and gives replayable provenance per game.
- **Real bugs were found by running Godot.** The builder's three (percent unique names, quoting, script id drift) came from the real engine and became lints plus tests. That instinct is right; it just stopped one rung short.
- **The kibitz loop works.** Append-only critique, backtalk adjudication, scope locks that held (no juice in the depot, Phases 6+ parked). The 30-minute watcher with a strict "unknown header = skip" gate is the right shape.

---

## 4. Critical finding: `script` is emitted in the middle of the property list

**Symptom.** In the real engine, the Player node of every depot game ignores most of its exported properties. Probe on `runner`: `.tscn` says `mode = 1`, Godot reports `mode = 0` while `speed = 5.5` was applied. With `mode = 0` and headless input zeroed, the player stands still forever.

**Cause.** `gamecraft-godot/src/export.rs` sorts property keys alphabetically (line 160). The `script` key lands in the middle. Godot applies `[node]` properties in file order, and exported script variables are only accepted after the script is attached, so everything that sorts before `script` (`axis`, `autofire`, `bullet_speed`, `checkpoints_to_win`, `cooldown`, `dialogue`, `fire_interval`, `fire_range`, `gravity`, `grid_h`, `grid_w`, `hp`, `jump_velocity`, `max_speed`, `mode`, `needs_key`) is dropped, while `speed` (after `script`) survives. The runner's Player block shows the shape:

```
gravity = 22.0
jump_velocity = 9.0
mode = 1
script = ExtResource("2_player")
speed = 5.5
```

Godot's own writer always emits `script` first.

**Blast radius.** A scan of all 12 `main.tscn` files finds properties before `script` on every game (1 to 11 nodes each). Per game, keys at risk: platformer (gravity, jump_velocity; its jump arc silently changes from 9.0/22.0 to the 7.0/20.0 defaults), runner/flappy/driving (mode, gravity, jump_velocity, checkpoints_to_win), pong/breakout (axis, max_speed, mode), sokoban/maze/chess (grid_w, grid_h, mode), talk-inventory (dialogue, needs_key, grid, mode), topdown-shooter (autofire, bullet_speed, fire_interval, mode, hp), tower-place (cooldown, fire_range, hp, mode). I confirmed the loss end to end on runner only. The other eleven follow from the same mechanism and the same file shape, but I did not run each fix; treat them as very likely, not proven.

**Why nothing caught it.**
- `scene_lint` checks structure (collision shapes, signal targets, resource ids). It has no notion of "property applied to a script variable".
- The Rust trace replay uses the intended parameters directly, so it passes regardless of what Godot does with the scene.
- The headless boot gate asserts silence. A scene that loads cleanly and does nothing is silent.

**Fix (about half a day including tests).**
1. Exporter: emit `script` first in each node, then the remaining keys sorted. Determinism is preserved.
2. New lint `ERR_PROPERTY_BEFORE_SCRIPT`, applied to the parsed scene, so hand-edited or third-party scenes are covered too.
3. Better still, a lint that parses the attached `.gd`'s `@export` vars and flags scene properties the script does not declare. That also catches typos.
4. Regression test in `gamecraft-godot` on the exact runner block above.
5. Regenerate the depot, diff, re-run the Godot probe (section 5).

---

## 5. The verification gap behind it

The depot pipeline has three gates and none of them executes gameplay in Godot:

1. `scene_lint`: structural, in Rust.
2. Trace replay: `sim.rs` re-implements a 2D kinematic stepper plus BFS and a Sokoban solver, and proves the games winnable **in that model**. The model is competent and small. After the ordering fix, runner hits the goal near frame 400 in Godot versus 415 predicted, so the physics mirror is close. But "close on one game" is one data point.
3. Headless boot: confirms no ERROR/WARNING lines.

What is missing is the rung that would have caught this: run each game in Godot with `--headless --fixed-fps 60`, drive the input from the trace, and assert the win line within N frames. Design notes from the code:

- In headless the player controller zeroes input for every mode except runner (`mode == 1` forces `dir = 1.0`), so 11 of 12 games cannot move without an injected driver. The controller already branches on `DisplayServer.get_name() == "headless"`; the clean version is a small harness that replays `trace.json` inputs via `Input.parse_input_event` or an autoload, not more headless branches in shipping scripts.
- The `Hint` Area3D nodes call `do_jump` on entry and are not headless-gated, so they exist in the shipped game and will auto-jump a human player. They serve the verifier, not the player. Gate them behind a verification flag or replace them with the harness above.
- Pong has no terminal win condition in practice (71 points in my run, no "win" line), which sits oddly against the plan's "vs simple AI" gate and the trace summary "2-0 in 215f".
- Plan said Godot 4.3 headful for 60 s; the project files and the builder ran 4.4 headless. Pick one and say which in the gate.

Suggested gate recipe: `just verify-depot` that, per slug, launches the harness, asserts the expected win marker, and fails the forge if any game misses. About 1 to 2 days to build properly; it is the highest-value investment in the repo right now.

---

## 6. Other findings

**Tests are thin where the risk is.** Forge is about 170 KB of source with 4 unit tests (all in `sim.rs`), an empty `tests/` directory, and no test that runs `forge_all`. The 12 game builders (`arcade`, `combat`, `puzzle`, `platformer`) have no direct tests; they are only covered by the forge's own post-hoc gates at generation time. Add a `forge_all` integration test into a temp dir with the same gates, so `cargo test` actually exercises the depot path.

**Docs have drifted behind the code.**
- `README.md` still describes the desktop app as a placeholder awaiting Phase 4 and omits forge and depot.
- `AGENTS.md` crate table omits `gamecraft-forge`.
- `CHANGELOG.md` ends at 0.2.1 (Phase 4b); Phase 5 is absent. Workspace version is still `0.1.0` while the changelog speaks of 0.2.1.
- `PLAN-phase5-depot.md` lists `command_run`, `trace_record` and `export_godot` as built MCP tools. The MCP server exposes `scene_lint` and `spatial_query` only; the builder's own note admits no `export_godot` exists and the forge calls the serializer in-process. For a repo billed as an MCP server, the 12-game generator is not reachable over MCP.
- Plan says the spatial gate includes "grounded at spawn, goal reachable via raycast walk". I found no such check in `spatial_query` or lint; reachability lives in the forge's Rust sim.

**No git yet.** `AGENTS.md` makes `git checkout -- .` the 3-strike circuit breaker, and the Phase 5 public gate requires "git init + clean tree". Today neither is possible. 103 `.bak` sidecars sit in the tree (about 88 in `forge/src`, from the file tool's automatic backups; `.gitignore` already excludes `*.bak`, so this is cosmetic, but a cleanup pass before the first commit is cheap). `data/searchstudio.sqlite3` (16 KB) does not look like it belongs here.

**Two builder contexts, divergent state.** At 13:30 (local time, labelled UTC) the watcher-run builder logged "0 of 12 slugs on disk, Phase 5 parked", and a later entry stamped 12:41 UTC logs "PHASE 5 DONE: 12/12". The interactive session evidently finished after the watcher's snapshot. It is harmless this time, but the log mixes local and UTC labels and the guardrail is only as good as the watcher's state read. Use one timezone (UTC, ISO 8601 with offset) in both logs.

**One test run hit a file lock** (`os error 32`) while OpenCode held the forge test binary. A second run passed. If two agents build in one `target/`, give the watcher its own `CARGO_TARGET_DIR`.

**Minor.** The desktop viewport draws rects instead of a true 12-edge wireframe (known, TODO in CHANGELOG). Warnings policy is sound; no suppressions seen beyond two scoped `too_many_arguments` allows in `sim.rs`.

---

## 7. Cross-check of the Antigravity assessment

Accurate on genesis, layering, kibitz protocol and the spatial-compiler versus world-compiler boundary. Two statements to qualify, both because it predates the forge:

- It says the verifier guarantees "grounded collision bounds, unbroken signal wires, camera coverage". `scene_lint` emits structural codes (missing collision shape, unresolved signal target, and now unresolved resource). Camera presence is checked in the forge, not in lint, and grounding is not linted.
- It reads "all green" as "ready". Green meant the Rust gates; section 4 shows real-engine behavior was not yet covered.

---

## 8. Roadmap reality check (Phases 6 to 11)

All sequencing locks are sensible. With AI-assisted pace, rough sizing:

| Phase | Honest read | Rough effort |
|---|---|---|
| Fix + Godot execution harness (not in plan; do first) | Required for any "verified" claim | 2 to 3 days |
| 6 Taste / juice | Plausible. Template-level GDScript passes plus a `feel_lint`. Needs the harness first, otherwise juice adds an unverified axis, exactly the risk the kibitzer named. | 3 to 5 days |
| 7 Universal IR and Bevy/web backends | A second backend is a real project, not a refactor; core's IR needs to stop being Godot-shaped. Steam Deck lint is small. | 4 to 7 days for Bevy MVP |
| 8 Economy and monetization | `economy_lint` is a contained math/state-machine job (1 to 2 days). The "sludge paradox" is strategy, not engineering; it needs no code. | 2 to 3 days plus strategy time |
| 9 to 11 Asset foundry, soul engine, mass ECS | Each is its own product (3D generation, runtime agents with embeddings and TTS, an ECS that cannot be expressed as a `.tscn` scene tree). The plan's single pipeline diagram hides that. Not a "next phase"; a separate research line. | Not estimable as written |

The unique asset here is not the roadmap, it is the claim "a generated game is verified before it ships." Make that claim true in the real engine for 12 games first.

---

## 9. Recommended next steps, in order

1. Fix export ordering, add `ERR_PROPERTY_BEFORE_SCRIPT` and an `@export` vs scene-property lint, regenerate the depot (half a day).
2. Build the Godot execution harness and `just verify-depot` with a win-marker assertion per game; decide Godot 4.4 as the target and update the plan (1 to 2 days).
3. Add a `forge_all` integration test so `cargo test` covers the depot path (half a day).
4. Resolve the hint-area design: keep out of shipping scenes or flag-gate them.
5. `git init`, clean `.bak` files, drop the stray sqlite file, update README/AGENTS/CHANGELOG/version, correct the PLAN's tool list (half a day).
6. Then, and only then, revisit Storytold and HN. Present the depot with the before/after of the ordering bug; it is a better story than "12/12 green".
7. Decide whether the forge belongs behind MCP (`export_godot`, `forge_game`); if the product is an MCP server for agents, it should.

---

## 10. Reproduction

```powershell
# gates
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo check -p gamecraft-core --target wasm32-unknown-unknown
cargo test --workspace

# real-engine check on a COPY of the depot (do not run inside the repo; Godot writes .godot/)
godot --headless --fixed-fps 60 --path <copy>\runner --script res://probe.gd   # probe prints Player pos/vel/won each second
# runner as exported: pos stays (0, 0.5, 0). With `script =` moved first: reaches goal near frame 400.
```

[STATUS: ASSESSMENT COMPLETE]
