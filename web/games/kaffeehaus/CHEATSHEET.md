# Kaffeehaus cheat sheet

All numbers are read from `sim.js`; if the code changes, this sheet may lag.
Time is game minutes; at 1x one real second is one game minute.

## Controls

| Key | Action |
|---|---|
| click guest button | Seat / Serve / Bill / Newspaper / Rush bill |
| click table | finish a Seat (gold ring = regular's table, blue = window) |
| Space | pause |
| 1 / 2 / 3 | speed 1x / 2x / 4x |
| S / M | sound / waltz |
| H | help |
| Esc | cancel a table pick |
| queue panel | arrow = do next, x = cancel |

## The day

- Open 07:00, doors close 18:30, Sperrstunde 19:30.
- Rent is EUR 90 a day. You start with EUR 150.
- Seating takes the order in the same visit. One serve trip carries a tray for
  up to 3 tables, nearest first.
- Coffee waiting on the pass for more than 9 minutes arrives "lauwarm" (-6 mood).

## Guests

| Type | Patience: door / order / serve / bill | Stays | Tip factor | Orders |
|---|---|---|---|---|
| Stammgast (regular) | 20 / 15 / 28 / 18 | 80-150 | 1.3x | their usual |
| Tourist (1 or 2) | 12 / 12 / 20 / 12 | 25-45 | 1.0x | coffee, cake 70% |
| Geschäftsmann | 8 / 8 / 16 / 8 | 12-20 | 1.1x | Brauner or Verlängerter |
| Studentin | 18 / 20 / 30 / 22 | 60-120 | 0.4x | Verlängerter, cake 15% |

- Past half their patience, mood drains. Past all of it they leave. At the bill
  stage tourists and students walk out without paying; regulars and business
  guests pay without a tip.
- Tip: mood 80+ = 12%, 60+ = 8%, 40+ = 4%, below = nothing (times the factor).
- **Second round:** a guest left in peace and still at mood 60+ may order again:
  regulars 75%, students 50%, tourists 40%, business 10%.

## Regulars

| Name | Usual | Table | Arrives | Paper |
|---|---|---|---|---|
| Hofrat Weiss | Melange | 4 | ~08:20 | yes |
| Frau Professor Lenz | Einspänner, Apfelstrudel | 2 | ~10:00 | yes |
| Herr Ostrowski | Kleiner Brauner | 7 | ~13:00 | no |
| Fräulein Mitzi | Verlängerter | 3 (window) | ~15:00 | no |
| Dr. Pöschl | Melange, Sachertorte | 8 | ~16:40 | yes |

Their own table: +10 mood. Any other table: -15. Newspaper: +12 and they stay
longer. Loyalty (0-5 hearts) at closing: +1 if content (mood 70+), -1 if rushed
or unhappy, -2 if they left angry. At 1 heart they come half the time; at 0,
never.

## Menu

| Item | Price | Brew |
|---|---|---|
| Melange | 4.90 | 3 min |
| Kleiner Brauner | 3.60 | 2 min |
| Einspänner | 5.40 | 4 min |
| Verlängerter | 4.20 | 3 min |
| Sachertorte | 5.90 | instant |
| Apfelstrudel | 5.20 | instant |
| Topfenstrudel | 5.20 | instant |

Cakes per day: 8 Sacher, 8 Apfel, 6 Topfen (x1.5 with the bigger vitrine).
Sold out = -8 mood for whoever wanted it.

## Rush bill

Frees the table now and pays the full bill now, but costs mood, scaled by how
much of the visit you cut short: regulars -30 minus up to 30 more, students -12,
tourists -8, business 0 (each minus up to 30). A rushed guest never orders a
second round, and a rushed regular loses a heart.

## Upgrades

| Upgrade | Cost | Effect |
|---|---|---|
| Bigger vitrine | 70 | 1.5x cake |
| Second brass machine | 140 | 4 brewing groups instead of 2 |
| Extra window table | 160 | table 11 |
| Piccolo | 220 | an apprentice who carries ready orders |

## Tactics that the bots proved

- **Deadline first.** Do whatever job lets the next guest down soonest, walk
  included. That one rule is what the good bot does, and it beats random play
  by a wide margin.
- **Bills are worth more than they look.** A walkout loses the whole bill, so
  take a "Zahlen, bitte!" ahead of a seat with the same urgency.
- **Seat by trips, not by door.** Business guests near the counter, students at
  the far tables, tourists at the window.
- **Never rush a regular.** Over five days, careful service ends near EUR 930
  with loyal regulars; rushing everyone ends near EUR 720 with almost none.
- **Buy the Piccolo early** once traffic grows; by day 4 one waiter is the
  bottleneck at midday.
