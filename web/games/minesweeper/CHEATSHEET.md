# Minesweeper AI: cheat sheet

| What | Value |
|---|---|
| Board | 9x9, 10 mines |
| Reveal | Left click a cell |
| Flag | Right click a cell |
| Chord | Double click an opened number: reveals its neighbours when enough flags are placed |
| First click | Never dies: the 3x3 around it is mine free |
| Take back | U or the Undo button, up to 200 steps |
| AI move | A or the AI button: one safe reveal, else one flag, else the lowest-density guess |
| Hint | H or the light bulb: outlines a safe cell or a mine for 1.5 s |
| HUD | Flags left and guesses made; status shows opened cells out of 71 |
| Gate | Headless: the solver clears the board, plus a sim where the solver must beat random picks |
| Win line | GameCraft: tour complete, win |
