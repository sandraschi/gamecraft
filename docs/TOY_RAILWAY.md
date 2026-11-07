# Toy railway

Status: first slice, 2026-11-07. Bevy runtime, branch `agent/claude-toy-railway`.

The model railway nobody can buy: two levels, two stations, two helices, a timetable that
actually runs, day and night, and a camera that goes from the whole table down to eye height
on a platform to watch the train you scheduled roll in.

## What is in this slice

| Part | Where |
|---|---|
| Track graph, block signalling, interlocking, timetables, layout validator | `crates/gamecraft-sim/src/rail.rs` |
| Bevy logic plugin (fixed 60 Hz, speed multiplier, pause, verdict) | `crates/gamecraft-bevy/src/railway.rs` |
| Rendering, camera continuum, day and night, boards, signals | `crates/gamecraft-bevy/src/render_railway.rs` |
| Binary with `--gate` and `--tour` | `crates/gamecraft-bevy/src/bin/railway.rs` |
| Landscape height field shaped by the track (tested headless) | `crates/gamecraft-bevy/src/terrain.rs` |
| Landscape mesh, trees, village, vineyard, cars | `crates/gamecraft-bevy/src/scenery.rs` |
| Catalog entry | `bevy/games/toy-railway/` |

Layout: Talheim (lower level, south straight) and Bergblick (upper level, 8 m up, north
straight), each with a main platform track and a passing loop. Two helices of 1.5 turns
(about 1.7 percent) join the levels. Twelve blocks, one signal each.

Working: Sparky, a steam tank engine, runs every 240 s and stops in the loops; the bullet runs
every 120 s on the mains and overtakes Sparky while it stands in the loop. Times were set from
measured run times (Sparky about 80 s a leg, the bullet about 52 s).

## Scenery (slice 2, 2026-11-07)

The flat table became a landscape. Natural ground is layered waves plus a hill inside the loop,
a ridge that carries Bergblick, low hills at the table ends, and flat pads for the village and
the station buildings; it eases down to the table frame so the edge shows no cliff. The track
then reshapes it: a stretch whose rails sit within 3 m of the natural ground (and whose
neighbours agree) becomes an embankment or cutting, everything else is a viaduct with at least
2.5 m of clearance, and the nearer track decides where the two meet, so an embankment ends at an
abutment. Piers stand on the real ground wherever the rails are more than 0.5 m above it.

Scenery is generated from fixed seeds: about 1500 low-poly trees (conifers on the heights,
broadleaf and a few birches lower down) clustered into forests, a village south of Talheim with
gabled houses along two streets, windows that light up at night, street lamps, a white church
with a green onion dome, seven cars on the main street, a vineyard on the hill, and patchwork
fields (wheat, plough, meadow, rapeseed) at the far ends.

Slice 3 added water and landmarks. A river runs from the south edge of the table under a
plate-girder bridge on the lower line (east of Talheim), then under the east helix viaduct into a
lake that the helix rings. The water is one plane at `WATER_Y` under the whole table; the land
stays above it everywhere except where the river and the lake are carved, so no water mesh has
to follow the banks. A castle ruin (keep, broken towers, gapped walls) stands on the hilltop, and
a fenced pasture inside the west helix has a red barn and a herd of Fleckvieh cows.

Terrain gates (`terrain::tests`): the bed sits 0 to 0.15 m under the rails (away from the
river), the ground never covers a rail anywhere, viaducts keep 2 m, Bergblick stands on the
ridge, the village is flat, there is water under the bridge and in the lake, and the land
elsewhere stays above the water plane.
The tour prints its frame rate; 60 fps (the vsync cap) with everything on, 2026-11-07.

## How safety works

- A train only moves into blocks it has reserved; a block has at most one owner; a block is
  released when the train's tail has left it.
- A train reserves ahead only as far as its braking distance plus a margin, and never past its
  next stop. A train standing at a platform reserves nothing until its departure time, so it
  never blocks the other train's route through the station.
- Movement authority ends 3 m before the first block it could not reserve (a red signal). Speed
  is capped by the braking curve to that point, by every speed limit ahead, and by distance per
  tick, so a head cannot pass its authority even with coarse ticks.
- The collision counter is geometric and independent of the reservation rules: it checks every
  pair of trains for overlapping extents on any block, every tick.

## Gates

| Gate | Where | Result on 2026-11-07 |
|---|---|---|
| 24 railway hours: no collision, no overrun, no deadlock, arrivals within 30 s, departures within 20 s | `rail::tests::a_day_runs_safe_and_on_time` (10 Hz) and `--gate` (60 Hz) | 2158 arrivals, worst arrival +0 s, worst departure +0 s |
| Same state hash on two runs | `two_runs_hash_the_same` | pass |
| Bad timetable (bullet 8 s behind Sparky) is held at red, never crashes, and the punctuality gate fails it | `a_bad_timetable_is_held_at_red_not_crashed` | pass |
| Steep helix (0.5 turns, 5.1 percent) refused for Sparky (3 percent), accepted for the Regio (6 percent) | `a_steep_helix_is_refused_for_steam_and_fine_for_the_regio` | pass |
| Bevy schedule and the bare sim agree tick for tick | `railway::tests::bevy_and_the_bare_sim_agree` | pass |
| Scripted tour: table, above Talheim, bullet arriving (day and night), table at night | `just railway-gate`, shots in `target/bevy/railway/` | looked at by eye before calling it done |

## Honest limits

- Two trains and one fixed timetable. There is no in-game timetable editor yet; timetables are
  `TrainPlan` data in `rail.rs`.
- The dispatcher is fixed-route: a train waits for its timetabled platform rather than taking
  another free one.
- Collision checking is per block; two trains on the two legs of a split right at the points are
  not checked for fouling.
- No crowd simulation (people are static), no sound yet, no gradient effect on speed (grades
  are only validated).
- Scenery is low-poly by design: no tunnels yet, houses have no chimneys or balconies, cows
  stand still, the water has no waves or reflections of the scene (only of the sky).
- TRELLIS.2 is not provisioned on Goliath (no weights, no Python environment as of
  2026-11-07), so every building here is procedural. Per `docs/TRELLIS2.md`, assets from it stay
  `pending-human-approval` for licence provenance before they ship. Trees are separate entities (about 7500 meshes), fine on Goliath, not
  tested on weaker GPUs.
- The tilt-shift look comes from Bevy's depth of field with an exaggerated sensor size at the
  overview; it is tuned by eye, not measured.

## Next slices

1. Timetable editor (a table in the HUD or a JSON file the viewer watches), with the
   validator and the 24-hour proof run on every change before it goes live.
2. More stock: diesel shunter, regional EMU, night sleeper; a fiddle yard on the lower level.
3. Sound from the existing synth: chuff synced to wheel turns, horn, an original arrival chime.
4. Platform crowd that boards and alights (boids from the murmuration sim).
5. More scenery: a tunnel (needs a layout change: the line would have to pass under the
   ridge), a road bridge over the river, buildings in bulk from TRELLIS.2 once provisioned and
   the licence is approved.
