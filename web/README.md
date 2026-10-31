# GameCraft web family (three.js)

The lightweight sibling of the Godot depot. Each game is a static page under
`games/<slug>/` (index.html + ES modules, three.js from `node_modules` via an
import map). No engine, no build step, runs in any browser including phones.

## Why it exists, and how it differs from the depot

The depot's rule is "the model writes a spec, never code", and everything in it
is asserted: lint, Rust trace replay, Godot boot. This family deliberately
breaks that rule. The model writes the whole game, the way Simon Willison's
October 2026 vibe-game experiments do, because that is where models are now
strong (three.js geometry, game feel, small story content).

What we can still assert, we do. Every game must expose `window.__gamecraft`
and pass `scripts/verify.ts`:

1. Boots in headless Chromium (WebGL via SwiftShader) with zero console errors.
2. Wins its own scripted self-play tour inside the step budget and prints the
   `win_marker` from `game.json`. Checkpoints along the tour assert real state
   (which layer is drawn, wall opacity, floor height), not just position.
3. Writes `thumb.png` and one screenshot per checkpoint to `shots/`.

What this does NOT prove: that the game is fun, or that the code is good. It
proves the game loads, the systems the tour touches behave, and a human can
reach the end. That is a weaker claim than the depot makes, on purpose.

## The hook contract

```js
window.__gamecraft = {
  ready,              // true once the scene is built
  state(),            // { steps, won, failed, checkpoint, checkpoints, log, ... }
  autoplay(),         // start the scripted tour
  step(n),            // advance n fixed 1/60 s steps (stops early at a checkpoint)
  render(),           // draw one frame
  selfTest(),         // optional: { pass, checks, summary } for games with a pure sim
};
```

A game with its own simulation (Kaffeehaus) should also prove its decisions
matter: `selfTest()` runs the sim headless with a good bot, a random bot and an
idle bot, and the gate requires `pass`. This is the closest thing to checking
"is there a game here" that a machine can do.

In `?verify=1` mode the page does not run its own loop, saves nothing to
localStorage, and creates no AudioContext; the verifier drives it.
`?autoplay=1` plays the same tour visibly at real speed.

## Commands

```powershell
cd web
bun install
bun run serve      # http://127.0.0.1:11258/
bun run verify     # gate, all games (Node 24; Playwright hangs under Bun)
```

`scripts/verify.bat` runs the gate detached and writes `scripts/verify.log`.

## Games

| Slug | Origin | Status |
|---|---|---|
| `mossbrook` | Willison's Emberwick prompt sequence, all 4 prompts, rebuilt from scratch | gate green: full story tour (14 checkpoints) plus stealth self-test; not yet hand-played |
| `kaffeehaus` | Original design: Viennese coffee house management | gate green including self-test; sound; not yet hand-played |

Each game folder has a `README.md` and a player `CHEATSHEET.md`
([Mossbrook](games/mossbrook/CHEATSHEET.md), [Kaffeehaus](games/kaffeehaus/CHEATSHEET.md)).
For building games here: [CHEATSHEET.md](CHEATSHEET.md) (commands, hook, gate
rules, patterns that worked, known traps). For the bigger picture, what a model
is good and bad at as a game designer: [../docs/LLM_AS_GAME_DESIGNER.md](../docs/LLM_AS_GAME_DESIGNER.md).
