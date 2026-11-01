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

## Shared harness (`games/_shared/gc.js`)

`harness({ checks, tick, bot, render, selfTest, extraState, winNote, paused })`
wires `window.__gamecraft`, the fixed 1/60 s loop, checkpoint markers and the win
marker; `wireHelp()` handles the help overlay (hidden in verify and autoplay).
Bots should act through the same functions a pointer would (project to screen,
raycast back), not by poking state, or the tour proves nothing about input.

## Music (`games/_shared/music.js`)

Generative, fully synthesized, no samples. A seeded composer writes 8-bar
sections in an A A B A form around a remembered motif, writes a new bridge every
round of the form and a new main tune every third. 14 styles; each game picks one
with `setupMusic('style', { sfx })`, which waits for the first gesture, is a no-op
in `?verify=1`, wires `#music-btn` and the M key, and remembers on/off across
games. `music.duck(seconds)` steps it back while a game plays sounds the player
must hear (Orgelbauer). Kaffeehaus and Mossbrook use `createMusic()` on their own
buses; Mossbrook switches between `mossbrook` (outdoors), `inn` and `deep`, and
leaves the troll's cave silent.

```powershell
node scripts/music-check.ts   # renders every style offline: no NaN/clipping/silence, sane level and register; WAVs to D:\Dev\repos\temp\music
node scripts/music-live.ts    # opens every game normally, clicks, checks a running AudioContext and no errors
```

The checks prove it plays cleanly, not that it is pleasant: listen to the WAVs.

## Known traps

- A button the bot `.click()`s is ignored while `disabled`; refresh the HUD right
  after state changes (Bento's Turn button stayed disabled after a pick; real
  players would have hit it too).
- Cameras built at load time need `updateMatrixWorld()` before any projection
  maths, or everything projects as if the camera sat at the origin.
- A canvas under an opaque HTML panel is invisible: give a poster portrait its own
  small renderer (Where is Benny).

- Playwright under the Bun runtime hangs; run the verifier with Node.
- Headless WebGL needs `--use-angle=swiftshader --enable-unsafe-swiftshader`.
- three.js r187 ships no minified build; import `three.module.js`, which pulls
  in `three.core.js` from the same folder.
- Overlays that start visible (help, title cards) end up in every screenshot
  unless verify mode hides them.
- Never use the em dash anywhere in this repo.
