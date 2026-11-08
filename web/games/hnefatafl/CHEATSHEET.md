# Hnefatafl: cheat sheet

| What | Value |
|---|---|
| Board | 11x11, Copenhagen rules: 24 attackers, 12 defenders and the king, attackers move first |
| Goal (defenders) | Move the king to any corner square |
| Goal (attackers) | Capture the king, ring in the king and every defender with an unbroken chain of attackers, or leave the defenders with no legal move |
| Moves | Every piece moves like a rook, any distance, no jumping. Only the king may stop on the throne or a corner; any piece may cross the empty throne |
| Capture | Sandwich an enemy piece between two of yours on a line (custodian). Corners and the empty throne count as hostile to both sides. The king can capture. Moving between two enemies is safe. Board edges are not hostile |
| King capture | Four attackers around it, or three plus the empty throne. The king cannot be captured on the board edge |
| Draw | The same position three times |
| Extra rules | Shieldwall capture; edge fort win (cautious test, may miss exotic forts, never invents one; needs 2+ defenders and a king that can move); encirclement win |
| Play | Click a piece, then a green square. N new game, S swap sides, L change level, U undo, H hint (AI suggestion), orbit with drag, zoom with the wheel |
| Levels | Easy: 1 ply with 25% random moves. Normal: 2 plies. Hard: 3 plies |
| URL options | `?side=attackers` or `?side=defenders`, `?level=` with easy, normal or hard |
| AI strength | Iterative-deepening alpha-beta with a hand-written evaluation (material, corner lines and forks, king region, cordon). Measured: at equal depth (2 v 2) the AI defenders escape in every game tried (5 of 5, 28 to 140 plies), so its attackers are the weaker side; attackers at 2 plies hold defenders at 1 ply to an undecided 250-ply game. Not a strong engine: no transposition table. |
| Gate | Headless: 39 rule checks on hand-built positions, then one full AI-vs-AI game (both sides 2 plies, fixed seed) that is replayed in a fresh engine and must reach the same final board |
| Win line | GameCraft: tour complete, win |