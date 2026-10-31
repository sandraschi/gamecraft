# Mossbrook: The Quiet Well

An angled top-down village RPG, rebuilt from scratch from Simon Willison's
Emberwick prompt sequence, all four prompts
(simonwillison.net/2026/Oct/28/vibe-game-development/).

Play: `just web-serve`, then http://127.0.0.1:11258/games/mossbrook/
Watch the bot play the whole story: add `?autoplay=1`.
Controls, map, stealth numbers and a spoiler walkthrough: [CHEATSHEET.md](CHEATSHEET.md).

## The story

Every lantern in Mossbrook is lit from the glow that rises from the old well at
dusk. Tonight it did not rise, and Captain Osk is sharpening his spear for the
troll. Two twists, no spoilers here beyond that; the seeds are all in the
villagers' small talk from the first three prompts.

About 15 minutes for a first playthrough, guided by an objective line and a
compass arrow. The journal (J) keeps what you have learned.

## The lantern (magic system)

| Flame | Where it comes from | What it does |
|---|---|---|
| Ember | any hearth (you start with one) | burns what is in the way |
| Glimmer | cave crystal | shows hidden marks and doors |
| Hush | the Moonring stones, south of the village | sleepers stay asleep |

Q cycles flames, E uses things. The lantern you carry lights the dark in the
active flame's colour.

**Stealth:** near the sleeping Keeper a stir meter fills. Without Hush he throws
you out of the cave in under three seconds; with Hush, walking, he sleeps on;
running with Hush still wakes him eventually.

## The world

Four times the area of prompt 3 (terrain doubled in both directions):

- the village (tavern with upstairs, smithy, weaver, bakery, archive, mill)
- cellars and the smugglers' tunnel, plus a cellar the village does not admit to
- Knuckle Crag and its cave, now with a passage behind the Keeper
- a river with a bridge and a fisher who has seen lights at night
- the Moonring, a circle of standing stones in the southern hills

Companions comment on new places; villagers' lines change as the story moves.

Keys: WASD/arrows, Shift run, E use/talk, Q flame, J journal, T time of day,
N sound, M music, wheel zoom, H help.

## Sound

All synthesised with the Web Audio API, no files. Ambience follows where you are:
wind with birds by day and crickets at dusk, the river as you get close, a hum
at the Moonring, crackle near any fire, drips and a low drone underground,
murmur in a full tavern. Footsteps change with the surface (grass, gravel,
wood, stone). The Keeper snores across the cave and a heartbeat rises with the
stir meter. Each flame, the glyph wall, the burning roots, the toss and the
ending have their own sound, and Odo's song is off-key on purpose. A quiet
lute tune plays outdoors. Starts on the first click or key; off in verify mode.

## Verification

- **Story tour:** the bot plays the whole story through the same movement and
  interaction code a player uses, and must reach the end. 14 checkpoints assert
  story stage, learned flames, the hidden door actually opening for collision,
  layer visibility, and that it was never thrown out of the cave. Last run:
  11,537 steps (about 3 minutes of play at bot speed), zero console errors.
- **Self-test (stealth must bite):** Ember walking gets you thrown out (2.7 s),
  running is worse (1.7 s), Hush walking is safe, Hush running is still risky
  (8.3 s).

## Honest limits

- Not hand-played yet. Route, pacing and puzzle clarity are bot-tested only.
- Most of the new area is landscape: the river, bridge, hut and Moonring are the
  points of interest out there; the rest is hills and about 700 trees.
- The bot passed on its first full run. The self-test above was added to make
  sure that was not because a mechanic did nothing.
- Sound is untested by ear; nothing in the headless gate can hear.
- Underground scenes at the default dusk lighting are dark in screenshots.
