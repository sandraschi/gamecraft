# GameCraft assessment, 2026-11-07

- Date: 2026-11-07T12:27:56+01:00
- Author tag: Claude (Claude Code harness, cloud session linked to Goliath)
- Repo state: branch `agent/opencode-trellis2` at `e2f6fe7`, `main` at `6091c8a`, tag `v0.13.0`. Working tree dirty: 13 modified, 16 untracked (before this file).
- Role: kibitzer. Read-only review. This file is the only write. No builds run, no `target/` touched, no other file edited, nothing staged or committed.
- Previous rounds: `ASSESSMENT-claude-2026-10-29.md`, `-r2`, `ASSESSMENT-antigravity-2026-10-29.md`, `KIBITZ-assessment-2026-10-31.md` (all at repo root).

## Method and its limits

Static only: `git status`, `git log --all`, `git diff`, file reads, byte counts. I did not run `cargo`, `just verify-depot` or `just web-verify`, because all three write into the repo (`target/`, depot outputs, `verify.json` and shots). Every gate statement below is therefore either cited from the record or marked PREDICTED. The engine is still the judge; nothing here replaces a run.

## Verdict in one paragraph

The project is in good shape as code and in poor shape as a repository. The habits that made the first week work (spec-not-GDScript, engine-side win assertions, honest README numbers) still hold in what is committed. But since 2026-11-02 the repo has drifted into a multi-agent working tree that nobody owns: five days of uncommitted work from at least two agents, a feature branch whose name no longer matches its content, a `main` eight commits behind, and a kibitz loop silent since 2026-10-31. The depot win gate has not moved in a week (11 of 15). The next useful work is not a new game; it is a cleanup-and-commit pass, then the four autopilots.

## 1. Repository hygiene (highest priority)

1.1 Uncommitted work is now the main risk. 29 paths dirty, oldest mtimes 2026-11-02 21:44 (`web/new.html`) and 2026-11-04 11:09 (`nekomimi.rs`, `docs/3D_STACK.md`). Only the nightly `refs/wip/*` snapshots protect it (last one 2026-11-07 04:02). Among the untracked items are nine whole web games: `detective-case`, `hnefatafl`, `jenga`, `minesweeper`, `rubiks`, `sudoku`, `sudoku-color`, `sudoku-letters`, `trichess`. That is a large share of the web family sitting outside any branch.

1.2 Ownership is unrecorded. AGENTS.md asks each session to say at the end what it left uncommitted and whose it is. No worklog after `WORKLOG_2026-11-02.md` exists. From content and timestamps the dirty tree looks like at least three threads: (a) VRM/Nekomimi plus `3D_STACK.md` (11-04), (b) sokoban-vault, flight barn texture, chess rename, maze/sokoban view changes (forge), (c) web `new.html` plus presets and the `/api/new/start` stub. Ask: whoever owns each thread commits it on its own branch with named paths, or Sandra says which threads to drop.

1.3 Branch drift. `agent/opencode-trellis2` now carries TRELLIS work plus bootstrap, the desktop auth modal, README status block, licenses and CONTRIBUTING. None of the last eight commits are on `main`. Suggest: merge it into `main` once its own tree builds clean, and start the next threads on fresh `agent/<tool>-<topic>` branches rather than piling unrelated work onto a branch named for one feature.

1.4 Line endings, root cause still open. `.gitattributes` contains only `.githooks/* text eol=lf`; there is no `* text=auto eol=lf`. Global `core.autocrlf` is `true` (from `C:/Users/sandr/.gitconfig`), while AGENTS.md says the owner setting should be `input`. Result on disk: tracked files check out CRLF (README, serve.ts, puzzle.rs, library.rs), new tool-written files are LF (`nekomimi.rs`, `3D_STACK.md`, `AGENTS.md`), and `KIBITZ.md` is genuinely mixed (204 CR vs 270 LF). Per AGENTS.md I am telling, not fixing: adding `* text=auto eol=lf` plus a one-time `git add --renormalize .` is an owner decision because it touches every file. Do it on a clean tree, in a commit of its own, and switch the global setting to `input` at the same time.

1.5 Noise. 54 `*.bak` files outside `target/` (fileops sidecars). All gitignored, so harmless to git, but they pollute `rg` hits and listings for agents. Periodic sweep; not urgent.

## 2. Findings in the uncommitted code (review of `git diff`)

2.1 PREDICTED clippy red: `apps/gamecraft-desktop/src/library.rs:169` has `((scroll.available_width() / per).floor() as usize).max(1)`. The repo bans `as` and runs pedantic with `-D warnings`; this trips `cast_possible_truncation` and `cast_sign_loss`. Same class as the four `economy.rs` casts on 10-29. The layout fix itself (top-aligned rows instead of `horizontal_wrapped` stairstepping) is right and the comment explains why. Snapshot PNGs were regenerated with it, so the snapshot test probably passes; clippy will not.

2.2 `bevy_vrm` added as a non-optional dependency of `gamecraft-bevy`. The crate's own Cargo.toml says the default build is logic-only and `cargo test -p gamecraft-bevy` never builds a renderer. `bevy_vrm` depends on `bevy` and `gltf`; through feature unification it very likely enables render features in the default build, which breaks that promise and slows every workspace test (the lockfile diff also bumps a `windows` dependency 0.61 to 0.62). Ask: `optional = true`, add it to the `render` feature next to the `nekomimi` bin's `required-features`. Check with `cargo tree -p gamecraft-bevy -e features` before and after.

2.3 `sokoban-vault` (`build_sokoban_9x9`). Naming is inconsistent three ways: function says 9x9, doc comment and solver call say 7x7, the web preset of the same name asks for 9x9 with 5 crates. More important is cost. The 10-31 backtalk measured 4 crates on 7x7 at over 1.2M states and 184 s; this layout is 4 crates on 7x7 with a 10M state cap, and `forge --all` builds every game twice for the determinism check. Expect `forge --all` to go from seconds to several minutes, every time. Either embed the solved move list with a cheap replay check in the forge, or land the A*/heuristic solver first. Also: `GAMES` now has 14 entries while the doc comment above it still says "All twelve".

2.4 View boxes look swapped. In the same diff `build_maze_with` view went 8 to 6 and `build_sokoban` went 6 to 8. The maze is 15x15 and the tutorial sokoban is small, so this reads like a find-and-replace slip. Look at both regenerated `thumb.png` / `play.gif` before committing.

2.5 Flight barn texture. The removed comment said textures need Godot's import pass and flat materials keep generated games error-free. The new code puts `barn_pbr.png` (TextureCraft output, via `include_bytes!`) on barns when `textured` is on. Right direction and what Sandra asked for, but the old warning has to be disproved, not deleted: run the flight headless boot with `textured` on and confirm zero ERROR lines. Also check the forge binary size growth from embedded PNGs.

2.6 Web `new.html` and `/api/new/start`. The endpoint is an honest stub (501, says so), which is the right way to ship it. Nits: the 501 message contains an em dash, which the fleet rule bans in every file; and the presets live as a const in `serve.ts` rather than as data next to `new.html`, so preset edits keep touching the server.

2.7 `docs/3D_STACK.md` looks misfiled. It calls itself the source of truth for `ai-games-collection` and `gamesapp`, and refers to `games/shared/physics-world.js`, which does not exist here (gamecraft has `web/games/_shared`). Move it to the repo it describes or rewrite its scope line. It also contains em dashes.

## 3. Status against the open queue

| Item (source) | Status today |
|---|---|
| Depot in-engine wins (10-29 reassess 3) | 11 of 15 per README, unchanged since 10-31. Depot now has 16 folders with `sokoban-vault`, so the denominator changes when it lands. |
| Autopilots for breakout, flappy, topdown-shooter, tower-place (10-31) | Not started as far as the tree shows. Still the single most valuable engineering item. |
| `verify.ts` DEP0190 one-liner (10-31, deferred for ownership) | Still open. `web/` is still not clean, so the same blocker applies. |
| `scene_verify` over MCP (10-31) | Declined by builder (self-sufficiency rule). I agree. `create_game` and `economy_lint` over MCP remain the right pure-Rust next tools and are not landed. |
| Plane 2 live attach | Not started. Correctly fenced. |
| Kibitz loop | Dormant since 2026-10-31 03:37. If retired, say so in the log; if not, the next builder should read this file. |

## 4. Web family

22 game folders plus `_shared`. By file presence: 3 games have no `game.json` (`jenga`, `rubiks`, `trichess`), so the verifier cannot gate them at all; `detective-case` has `game.json` but no `verify.json`, so it has never passed. 7 games lack the `CHEATSHEET.md` AGENTS.md makes mandatory (`detective-case`, `hnefatafl`, `jenga`, `minesweeper`, `rubiks`, the three `sudoku*`, `trichess`). All are in the untracked set, probably one agent's batch that stopped before the contract step. Before committing: `game.json`, `window.__gamecraft`, a passing `just web-verify` on those slugs, and a look at `shots/`.

The puzzle classics (sudoku, minesweeper, rubiks, hnefatafl, trichess) differ from kaffeehaus and mossbrook: their win is a solver question, not a tour. They are the natural place for a negative self-test (a random clicker must not solve sudoku), which is what made the web gate meaningful in the first place.

## 5. Strategic kibitz

- Breadth is outrunning proof again. In one week the repo gained Bevy games, VRM avatars, TRELLIS, a new-game web page, nine web games, a 16th depot game and textures, while the four unproven depot games and the MCP tool gap did not move. Same pattern the 10-29 reassessments flagged. Not a reason to stop exploring, but every new surface should arrive with its own gate, the way `mossbrook` did.
- There are now three game runtimes (Godot depot, Bevy, web three.js) plus a fourth stack described in `3D_STACK.md`. One README paragraph saying which runtime is the default for new work, and why, would stop agents choosing per session.
- The multi-agent rules are good but not followed at session end. Cheap enforcement: a `just handoff` recipe that prints `git status`, the last five commits, and appends a stamped line to the current worklog. Agents follow recipes better than prose.

## 6. Suggested order

1. Owners commit or drop their threads (1.1 to 1.3), each on its own branch, named paths only. Fix 2.1 and 2.2 first; check 2.4 and 2.5 visually and headless.
2. Decide the line-ending fix (1.4) and do it alone on a clean tree.
3. Merge to `main`, run the full gate plus `just verify-depot` and `just web-verify` on the merged tree, record the numbers.
4. The four autopilots: depot to 15 of 15 (16 of 16 with the vault).
5. Bring the untracked web games up to contract (section 4).
6. Then `create_game` and `economy_lint` over MCP.

No item here was verified by running code. The numbers to trust are the ones the next gate run prints.
