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
bun run pwa-check  # PWA gate: manifests, icons, service worker, every game offline
bun run pwa-icons  # regenerate install icons from thumb.png (after a thumb changes)
```

`scripts/verify.bat` runs the gate detached and writes `scripts/verify.log`.

## Install and offline (PWA)

Every game installs as its own app and plays offline once installed; the arcade
index installs as "GameCraft". Nothing in a game folder is PWA boilerplate:

- `serve.ts` injects the manifest link, icons and `/pwa.js` into each page at serve time.
- `pwa.ts` generates the manifests (from `game.json`) and the service worker. The worker
  precaches every game, `_shared`, three.js and the addons the games import, and is
  **network-first**: a running server always wins (edits show at once), the cache only
  answers when the network is gone. Any file change gives a new cache name.
- Icons are `games/<slug>/icon-192.png` / `icon-512.png` (cropped from `thumb.png`) and
  `icons/icon-*.png` (a 2x2 mosaic) for the arcade. A new game needs `bun run pwa-icons`.
- Under `?verify` the worker is not registered, so the gameplay gate never hits a cache.

**On a phone.** Service workers need a secure origin: `http://127.0.0.1` counts, a
phone on the LAN does not. The server binds 127.0.0.1 only, so expose it over HTTPS
on your tailnet (private, not public like Funnel):

```powershell
tailscale serve --bg 11258     # https://<machine>.<tailnet>.ts.net/  -> 127.0.0.1:11258
```

Open that URL on the phone while the server runs, then Share > Add to Home Screen
(iOS) or Install app (Android). After one online visit everything is cached, and the
games keep working with Goliath off.

## Games

| Slug | Origin | Status |
|---|---|---|
| `mossbrook` | Willison's Emberwick prompt sequence, all 4 prompts, rebuilt from scratch | gate green: full story tour (14 checkpoints) plus stealth self-test; not yet hand-played |
| `kaffeehaus` | Original design: Viennese coffee house management | gate green including self-test; sound; not yet hand-played |
| `spot-the-difference` | Calm batch 2026-11-01: one room rendered twice, 5 fresh differences per round | gate green incl. self-test; not yet hand-played |
| `tidy-the-room` | Same room: six things on the floor, a photo of where they live | gate green incl. self-test; not yet hand-played |
| `where-is-benny` | Funfair crowd, up to 44 dogs, five shepherd look-alikes, one Benny | gate green incl. self-test; not yet hand-played |
| `kintsugi` | Turn and flip shards home, cracks fill with gold | gate green incl. self-test; not yet hand-played |
| `shadow-sculpture` | Kage-e: turn a junk heap until its shadow is a picture | gate green incl. self-test; not yet hand-played |
| `lantern-river` | Toro nagashi: breathe lanterns into a shrine cove; pure sim | gate green incl. self-test; sound; not yet hand-played |
| `hoshi` | Draw constellations on a real-coordinate sky, Western and Japanese names | gate green incl. self-test; not yet hand-played |
| `bento` | Polyomino packing, generator plus independent solver | gate green incl. self-test; not yet hand-played |
| `gassi` | Route memory with Benny in a stylized Alsergrund | gate green incl. self-test; not yet hand-played |
| `organ-builder` | Orgelbauer: order pipes by ear so the organ plays a tune; WebAudio voice | gate green incl. self-test; sound; not yet hand-played |
| `robot-sorting` | Boomy sorts parcels with first-match IF/THEN rules | gate green incl. self-test; not yet hand-played |

The calm batch shares `games/_shared/` (no `game.json`, so the arcade and the
verifier skip it): `gc.js` owns the hook contract, fixed-step loop, verify mode,
seeded rng, picking helpers, storage and a tiny synth; `ui.css` the common look;
`room.js` the procedural room used by Spot the Difference and Tidy the Room.
Every one of them has a self-test and at least one deliberate wrong move in its
tour (a negative check that the mechanic actually bites).

Each game folder has a `README.md` and a player `CHEATSHEET.md`
([Mossbrook](games/mossbrook/CHEATSHEET.md), [Kaffeehaus](games/kaffeehaus/CHEATSHEET.md)).
For building games here: [CHEATSHEET.md](CHEATSHEET.md) (commands, hook, gate
rules, patterns that worked, known traps). For the bigger picture, what a model
is good and bad at as a game designer: [../docs/LLM_AS_GAME_DESIGNER.md](../docs/LLM_AS_GAME_DESIGNER.md).
