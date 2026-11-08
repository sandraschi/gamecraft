# Hnefatafl: cheat sheet

Status: skeleton. This is a scripted tour, not a playable game yet.

| What | Value |
|---|---|
| Board | 7x7, king on the throne at the centre, corners gold |
| Input | None. There are no click or key handlers; the board does not react to you |
| Tour | The king slides rook-wise to (3,1), (1,1), (0,1), (0,0) and escapes to the corner |
| Pieces | Only the king exists in the code; no attackers and no captures yet |
| Rules in help | The ? panel describes the full game (defenders 8 + king, attackers 16, sandwich capture, 11x11); the code does not implement it |
| Next | Port the 11x11 rules, throne and shield-wall from ai-games-collection hnefatafl.js |
| Gate | Headless self-play checks board-7x7 and escape |
| Win line | GameCraft: tour complete, win |
