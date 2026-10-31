# Web family developer cheat sheet

## Commands

```powershell
cd web
bun install                     # three.js + playwright
bun run serve                   # arcade on http://127.0.0.1:11258/
node scripts/verify.ts          # gate, all games (Node 24, not Bun)
node scripts/verify.ts <slug>   # gate, one game
scripts\verify.bat              # gate detached, output in scripts\verify.log
```

From the repo root: `just web-serve`, `just web-verify`.

| URL suffix | Effect |
|---|---|
| `?autoplay=1` | the game plays its own tour, visibly |
| `?verify=1` | no loop, no audio, no saves; the verifier drives it |
| `?time=night` | Mossbrook start time (dawn, noon, dusk, night) |

## Adding a game

1. `web/games/<slug>/index.html` with an import map:
   `{ "imports": { "three": "/vendor/three/three.module.js" } }`
2. `game.json`: `slug`, `title`, `win_marker`, `thumb_checkpoint`, `step_budget`.
3. Expose the hook (below), print `GameCraft: <win_marker text>` on success.
4. Run the gate, then **open `shots/` and look**. Two harness bugs were only
   ever found that way.
5. Add a `README.md` and, for players, a `CHEATSHEET.md`.

## The hook

```js
window.__gamecraft = {
  ready,        // true once built
  state(),      // { steps, won, failed, checkpoints, log, ... }
  autoplay(),   // start the scripted tour or bot
  step(n),      // n fixed steps; must stop early at a new checkpoint
  render(),     // draw one frame
  selfTest(),   // optional: { pass, checks, summary }
};
```

Console markers the verifier reads: `GameCraft: ready`, `GameCraft: checkpoint
<name>`, the win marker, `GameCraft: check failed ...`, `GameCraft: autoplay
stuck ...`.

## Gate rules

A game passes only if: zero console errors and page errors, the win marker is
printed within `step_budget`, and `selfTest().pass` when present. Results land
in `games/<slug>/verify.json`; `shots/` is cleared and rewritten every run.

## Patterns that worked

| Pattern | Where | Why |
|---|---|---|
| Pure sim module, render on top | `kaffeehaus/sim.js` | lets `selfTest` run hundreds of days in seconds |
| Good bot vs random bot vs idle | Kaffeehaus self-test | proves decisions matter; caught four broken designs |
| Negative self-test | Mossbrook stealth | a tour that passes first time may mean a mechanic does nothing |
| Checkpoints assert state | both tours | "arrived at x,z" proves little; "wall faded, layer hidden" proves the system |
| Collision spaces + walk rects | `mossbrook/world.js` | floors, cellars and caves without a physics engine |
| Hysteresis on space switches | cave mouth | no flicker at the boundary |
| Audio only after a gesture | both `audio.js` | browser rule; also keeps verify mode silent |

## Known traps

- Playwright under the Bun runtime hangs; run the verifier with Node.
- Headless WebGL needs `--use-angle=swiftshader --enable-unsafe-swiftshader`.
- three.js r187 ships no minified build; import `three.module.js`, which pulls
  in `three.core.js` from the same folder.
- Overlays that start visible (help, title cards) end up in every screenshot
  unless verify mode hides them.
- Never use the em dash anywhere in this repo.
