# Hnefatafl: cheat sheet

| What | Value |
|---|---|
| Board | 11x11, Copenhagen rules: 24 attackers, 12 defenders and the king, attackers move first |
| Goal (defenders) | Move the king to any corner square |
| Goal (attackers) | Capture the king, or leave the defenders with no legal move |
| Moves | Every piece moves like a rook, any distance, no jumping. Only the king may stop on the throne or a corner; any piece may cross the empty throne |
| Capture | Sandwich an enemy piece between two of yours on a line (custodian). Corners and the empty throne count as hostile to both sides. The king can capture. Moving between two enemies is safe. Board edges are not hostile |
| King capture | Four attackers around it, or three plus the empty throne. The king cannot be captured on the board edge |
| Draw | The same position three times |
| Not implemented | Shieldwall capture, edge fort win, encirclement win |
| Play | Click a piece, then a green square. N new game, S swap sides, L change level, U undo, H hint (AI suggestion), orbit with drag, zoom with the wheel |
| Levels | Easy: 1 ply with 25% random moves. Normal: 2 plies. Hard: 3 plies |
| URL options | `?side=attackers` or `?side=defenders`, `?level=` with easy, normal or hard |
| AI strength | Alpha-beta search with a hand-written evaluation. The defenders are the stronger AI side: at equal depth its attackers usually lose to a king escape. |
| Gate | Headless: 31 rule checks on hand-built positions, then one full AI-vs-AI game (attackers 2 plies, defenders 1 ply, fixed seed) that is replayed in a fresh engine and must reach the same final board |
| Win line | GameCraft: tour complete, win |