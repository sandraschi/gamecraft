# Kaffeehaus (Cafe Morgenstern)

Viennese coffee house management. You are the Herr Ober: seat guests, serve,
bring newspapers, collect bills. No reflexes needed; pause any time.

Play: `just web-serve`, then http://127.0.0.1:11258/games/kaffeehaus/
Watch the bot: add `?autoplay=1`.
Every number in the game (patience, prices, tips, regulars, upgrades) and the
tactics the bots proved: [CHEATSHEET.md](CHEATSHEET.md).

## How it plays

- Guests queue at the door. Click **Seat**, then a table (gold ring = a regular's
  favourite, blue = window). Seating includes taking the order.
- Orders go to the brass machine (2 groups, 4 with the upgrade) and the cake
  vitrine (stock runs out). When a whole order is ready, **Serve**. One trip
  carries a tray for up to three tables.
- **Bill** when they ask. **Rush bill** frees a table early: business guests do
  not mind much, regulars remember it, and a rushed guest never orders a second
  round.
- The ring next to each name is patience. Guests who run out leave; at the bill
  stage some walk out without paying.
- Five named regulars with usual orders and tables. Loyalty (0 to 5 hearts)
  carries across days; at 0 they stop coming.
- Closing shows takings, tips, rent, stars and how each regular felt. Upgrades:
  second machine, bigger vitrine, a Piccolo who carries ready orders, an extra
  window table. Progress saves in localStorage. "New cafe" resets.

Keys: Space pause, 1/2/3 speed, S sound, M waltz, H help, Esc cancel table pick.

## Sound

All synthesised with the Web Audio API, no files: room murmur scaled to how full
the room is, machine hiss while brewing, door bell, chair, cup clink, paper,
register, grumble, and a quiet music-box waltz in D. Starts on the first click or
key (browser rule); off entirely in verify mode.

## Files

| File | Role |
|---|---|
| `sim.js` | The whole game as pure, seeded logic. No three.js, no DOM. Also the bots and `selfTest()`. |
| `main.js` | three.js room, characters, HUD, input, save, verifier hook. |
| `audio.js` | Procedural sound and the waltz. |
| `index.html` | HUD markup and styles. |

## Verification

`selfTest()` (run by the gate) plays 5 seeds each with a deadline-first bot, a
random bot and an idle bot:

| Check | Last result |
|---|---|
| Day 1: good bot >= 3 stars, beats random on takings and stars | 3.06 vs 1.32 stars |
| Busy day (day 4 traffic, no upgrades): good bot beats random | takings +11%, stars 2.56 vs 1.14 |
| 5-day campaign: good bot cash > random x 1.25, random loses regulars | EUR 931 vs 723, loyalty 20 vs 1 of 25 |

The scripted tour plays one full day with the good bot and asserts state at six
checkpoints (first seated, 09:00, 12:30, 16:00, 18:30, day end); last run 2.8
stars, EUR 140 profit, zero console errors.

History worth knowing: the self-test rejected four versions before it passed
(waiter walked into the street, patience shorter than the room, no tray
batching, rushing was free). The busy-day takings margin was lowered from 15% to
5% after measuring; under overload rushing genuinely buys throughput, so the
campaign check carries the "skill pays" claim.

## Known gaps

- Not hand-played yet. Balance is bot-tested, not human-tested.
- Guests walk through furniture (straight-line movement).
- Chair backs may sit oddly; screenshots were checked by pixel statistics only.
