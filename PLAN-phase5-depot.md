# PLAN Phase 5 - Twelve Playable Games Depot (DRAFT, not started)

Status: DRAFT 2026-10-29. Do NOT start until Phase 4a viewport is [STATUS: SUCCESS].
Owner: builder chat. Kibitzer may comment via KIBITZ.md only.

## Goal

Prove GameCraft with 12 playable Godot 4.3 exports. No storytold move, no HN post until 12/12 boot in stock Godot with zero manual fixups.

## Depot layout

gamecraft/depot/games/<slug>/ per game:
- project.godot + .tscn/.tres (deterministic export, hashed uid://)
- lint.json (scene_lint, zero errors)
- trace.json (trace_record 60-frame input playback, start -> goal reachable)
- thumb.png + play.gif (viewport capture, not marketing)
- README.md (controls, win condition, seed)

## The twelve (deliberately boring, proves generality)

1. platformer - run + jump to goal
2. topdown-shooter - WASD + shoot, 3 waves
3. sokoban - push 3 crates to targets
4. breakout - paddle + 20 bricks
5. pong - vs simple AI
6. maze - find exit, 60s timer
7. runner - auto-scroll, jump gaps
8. tower-place - place 3 towers, survive wave
9. talk-inventory - NPC talk + pickup + door
10. driving-box - box car, 3 checkpoints
11. flappy - hold to lift, 10 pipes
12. chessboard-move - select + move piece legally

## Acceptance gate per game

- scene_lint: 0 errors (missing shape, unresolved signal, bad res://, no camera all fail)
- spatial_query: player AABB grounded at spawn, goal reachable via raycast walk
- trace_record: scripted inputs reach win state in <= 600 frames
- headful: opens in clean Godot 4.3, runs 60s without errors in debugger
- export determinism: re-export twice, byte-identical .tscn except timestamp header

## Public gates

- Storytold: only after 12/12 green + just check-all green + git init + clean tree. They take repos with passing gates, not claims.
- HN Show: title "GameCraft: 12 Godot games generated with a Rust spatial verifier". Body: 5 failure modes, one lint JSON before/after, repo + depot link, ask for broken scenes. Post once, morning ET, answer comments with traces.

## Non-goals

- No fun-linter claims. Playable + correct, not "fun". World compiler is horizon, see on-metacraft side-red-letter.
- No new MCP tools in Phase 5. Use scene_lint / spatial_query / command_run / trace_record / export_godot as built.
