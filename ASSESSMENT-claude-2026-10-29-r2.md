# GameCraft - Reassessment (Claude, round 2)

- **Date:** 2026-10-29, about 18:35 CET (first assessment was 13:48 CET, see `ASSESSMENT-claude-2026-10-29.md`)
- **Author tag:** Claude (Sonnet 5.5), via Claude Desktop on Goliath
- **Target:** `D:\Dev\repos\gamecraft` (still local only, no git repo)
- **Scope:** read-only on the repo apart from this file. Godot runs were done on a fresh copy of the depot in `d:\dev\repos\temp\gc-depot-run3`. The runner script is `d:\dev\repos\temp\gc_godot_run3.ps1` and is safe to copy into a `just` recipe.
- **Caveat on timing:** `llm.rs` was modified at 18:32:45, during my run. The builder was mid-edit, so the red gates below may already be fixed.

---

## 1. Verdict

Strong response to round one, and the headline bug is fixed. The exporter now emits `script` first (`export.rs` lines 155 to 160, with a comment that explains exactly why). In a real Godot 4.4 run, 5 of 14 depot games now print `GameCraft: goal reached` headless, versus effectively none before. That is the right direction and it validates the fix on more than the one game I proved last time.

But "verified" still overstates reality. **9 of 14 games print no win in real Godot**, the strict gates are red at this moment, and the shipped games appear to play themselves. Feature growth (LLM-driven game creation, a 14th game, textures, desktop mesh viewer, 0.2.1 to 0.10.0 in one day) is outrunning the verification work that I called the highest-value item. Hold the public steps (Storytold, HN) until the engine-run assertion covers every game.

---

## 2. Progress since the first assessment

| Item | Then | Now |
|---|---|---|
| Exporter `script` ordering bug | present on 12 of 12 games | fixed in `export.rs`; depot regenerated at about 17:29 |
| Tests | 45 pass | **81 pass**, 0 fail (core 12, commands 6, godot 10+2, forge 14, mcp 22, sim 8, desktop 7) |
| Depot games | 12 | **14 directories**: added `flight` and `night-forest`; `driving-box` is now `driving` |
| MCP tools | 2 | **4**: `scene_lint`, `spatial_query`, `scene_inspect`, `export_scene` |
| New crate | none | `gamecraft-sim` (shared sim, 8 tests) |
| CLI | lint, query, uid, forge | adds `new` (prompt or `--from-spec` to compiled, gated game; local, OpenAI-compatible and Anthropic providers via `curl.exe`) |
| Desktop | placeholder-to-viewer | depot quick-select, Start Game, mesh renderer |
| `.bak` clutter | 103 files | 1 |
| Log timestamps | mixed local/UTC | ISO 8601 with offset adopted; old entries intentionally left as evidence |
| CHANGELOG | ended 0.2.1, no Phase 5 | through 0.10.0 |
| Open thread 2 in the changelog | n/a | "assert on goal reached after boot, not just no ERROR lines" - the builder named my main finding as its own open item |

Credit where due: the builder kept honest limits in the changelog. The LLM path says "the stub proves the HTTP contract, not a live model", and Start Game is listed as "not click-tested end to end - treat as unproven".

---

## 3. What I re-ran

| Check | Result |
|---|---|
| `cargo test --workspace` | green, 81 tests |
| `cargo check -p gamecraft-core --target wasm32-unknown-unknown` | green |
| `cargo clippy ... -D warnings` | **RED**: `clippy::single_match_else` at `crates/gamecraft-forge/src/llm.rs:251`; blocks the forge lib and its tests from compiling under clippy |
| `cargo fmt --all --check` | **RED**: one diff at `llm.rs:142` (a stray blank line) |
| Godot 4.4 headless, `--fixed-fps 60`, 3600 frames, all 14 games (copy) | 0 ERROR/WARNING lines in all 14; **5 print a win marker**; 9 do not |

The red gates are both in `llm.rs`, which was being edited while I ran. AGENTS.md says `just check` must pass before any task is marked complete, so the 0.10.0 changelog entry should be read as "green at its time of writing". Fix is a few minutes; I list it for completeness, not as a regression.

### Engine run, per game

| Game | Headless win marker | Note |
|---|---|---|
| platformer | `goal reached` | was dead in round one; now works |
| runner | `goal reached` | same |
| driving | `goal reached` | autoplay |
| flight | `goal reached` | autoplay |
| night-forest | `goal reached` | the AI-spec path; confirms the changelog claim |
| pong | none | 4 log lines, no win |
| breakout | none | silent |
| flappy | none | silent |
| topdown-shooter | none | silent |
| tower-place | none | silent |
| sokoban | none | silent |
| maze | none | silent |
| talk-inventory | none | silent |
| chessboard-move | none | silent |

Read this carefully: "no win marker" does not prove those nine are broken. They have no in-engine driver in headless, so they cannot self-play and may be fine for a human. It means they are **unproven in the engine**, and their README claims ("verified trace") rest only on the Rust model. The five that win are exactly the ones that carry `autoplay.gd`.

---

## 4. Findings

**4.1 Engine verification covers 5 of 14 games.** The Rust replay model is good, and round one showed it tracks Godot closely on runner (about 400 vs 415 frames), but a replay of your own re-implementation cannot prove the engine agrees. The nine unverified games need an in-engine driver: for grid games, replay the solver's move list as synthesized input; for breakout, pong, shooter and tower, an autoplay policy or a scripted input list from the trace. This is the work in my previous section 5, still open, and the builder agrees (thread 2).

**4.2 Autoplay ships enabled by default.** `autoplay.gd` has `enabled = true`, and each physics tick it calls `Input.action_press` on the hold action (for platformer, `ui_right`) and the jump action. It is not headless-gated, and `main.tscn` contains no override. By code reading, a human pressing F5 on platformer, runner, flappy-style or driving games would be fighting an autopilot that holds right and jumps gaps for them. I did not test this headful, so treat it as strongly suspected, not confirmed. Fix: ship `enabled = false`, and let the harness (and the desktop viewer if desired) turn it on through a command-line user arg or an environment variable. The depot README's "Controls" column currently describes behavior a player may not actually get.

**4.3 The gate story is still "silent equals pass".** `docs` and CHANGELOG say the boot test checks for no ERROR lines. The 5-of-14 result shows why that alone is weak: all 14 are silent. The fix is the assertion on `GameCraft: goal reached` (or the game's own win line) within N frames, per slug, as `just verify-depot`. My run script already does it for 14 games in about 45 seconds.

**4.4 Scope is ahead of proof.** In a single day the repo gained: LLM-to-game compiler with three providers, textures, a flight sim, an AI-spec game, desktop mesh rendering, MCP client wiring, a toolset plan, and horizon docs. That is impressive throughput, but every new archetype added to the depot without the engine-run assertion widens the unverified surface. The kibitzer's earlier scope lock ("no second unverified axis") has quietly eroded. I would pause feature work until 4.1 and 4.3 are closed. The prompt-to-game path in particular should not be advertised until the generated spec runs through the same engine assertion automatically.

**4.5 Hygiene still open.**
- No git repo. AGENTS.md still names `git checkout -- .` as the circuit breaker, and the Phase 5 public gate still requires `git init` and a clean tree.
- `depot_test_look/` at the repo root is a scratch directory (textures, scenes, a screenshot) from about 17:12.
- `depot/` now contains 3 `.godot` cache directories and about 20 `.import`/`.uid` files from Godot runs. `.gitignore` grew to 164 bytes; confirm it covers `.godot/` and decide whether `.uid` and `.import` files are committed (Godot 4.4 expects `.uid` files to be committed).
- `data/searchstudio.sqlite3` is still there and still looks unrelated.
- Workspace version is `0.1.0` while the changelog is at 0.10.0.
- Root `README.md` is untouched since 09:31 and still calls the desktop app a placeholder. The depot README says "thirteen" in the title, "All twelve" in the body, and the table lists 13; `night-forest` (the 14th) is not listed.

---

## 5. Scorecard on my round-one recommendations

| # | Recommendation | Status |
|---|---|---|
| 1 | Fix export ordering, add lint, regression test | **Done** for the exporter, confirmed in code and by 5 engine wins. A new `lint.rs` appeared at 14:40; I did not read it, so I cannot confirm a dedicated property-before-script lint. |
| 2 | Godot execution harness, `just verify-depot` | **Open** (thread 2). `just` has `depot-lint`, `depot-godot`, `godot-import`; none asserts a win. |
| 3 | `forge_all` integration test | Forge tests went from 4 to 14; I did not confirm one that runs the whole depot. |
| 4 | Take hint areas out of shipping scenes | Replaced in part by `autoplay.gd`, which has the same flaw in a new form (4.2). Runner README still says "hints auto-jump". |
| 5 | git init, clean `.bak`, docs, version | `.bak` cleaned, CHANGELOG current; git, version, README, stray files open. |
| 6 | Hold Storytold and HN | Still correct. |
| 7 | Forge behind MCP | Partly: `export_scene` added; `new` and `forge` are CLI-only. |

---

## 6. Recommended next steps

1. Fix `llm.rs` (clippy and fmt) so `just check` is green again. Minutes.
2. Make autoplay opt-in: `enabled = false` in shipped scenes, enabled by the harness. Half a day with regeneration.
3. `just verify-depot`: per slug, run Godot headless with `--fixed-fps 60`, assert the win marker within a frame budget, fail the build otherwise. Start from `gc_godot_run3.ps1`. Half a day for the 5 that pass, plus 1 to 2 days for in-engine drivers for the other 9.
4. Make the prompt-to-game path (`new`) call the same engine assertion before it reports success.
5. `git init` after cleaning `depot_test_look/`, the sqlite file and caches; bump the version; refresh README and the depot README. Half a day.
6. Resume feature work, and then take up Phase 6, only after 3 and 4 are green for all 14.

[STATUS: ASSESSMENT COMPLETE]
