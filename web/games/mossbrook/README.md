# Mossbrook

Angled top-down village RPG engine, rebuilt from scratch from Simon Willison's
Emberwick prompt sequence (prompts 1-3 of 4,
simonwillison.net/2026/Oct/28/vibe-game-development/).

Play: `just web-serve`, then http://127.0.0.1:11258/games/mossbrook/
Watch the tour: add `?autoplay=1`.

## What is in it

- Rolling terrain, six buildings (two with an upstairs), furniture throughout.
- Two companions who follow your trail and comment on new places.
- Ten villagers who wander and talk; a note to find in the tunnel.
- Cutaway: inside a building the roof goes, the camera-side wall fades, and the
  upper floor hides until you climb the stairs. Outdoors, any building between
  camera and player fades.
- Cellars under the tavern and the smithy, joined by a smugglers' tunnel.
- A mountain with a cave and a sleeping troll.
- Dawn / noon / dusk / night lighting, minimap, touch joystick.

Keys: WASD or arrows, Shift run, E talk, 1-4 time of day, wheel zoom, H help.

## Verification

The tour visits 8 checkpoints and asserts state at each: roof and upper floor
hidden with the wall faded (tavern), standing upstairs at the right height,
surface layer hidden underground (cellar, tunnel), back above in the smithy,
outdoors with roofs visible, cave layer drawn, and within 5 m of the troll.

## Not done

Prompt 4 ("turn this into an actual game": larger map, mechanics, plot, magic).
No sound. Not hand-played yet.
