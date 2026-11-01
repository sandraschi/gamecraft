# Bento

Pack food-shaped polyominoes into a lacquer box. Six sizes from 3x2 to 6x5. Each
box is cut from a full grid by random growth, so it always has an answer; pieces
come turned. A separate backtracking solver re-proves solvability in the self-test.

Self-test (6 sizes x 25 seeds): always fills the box, piece sizes in range, pieces
connected, solver agrees (worst case under 1000 nodes), something always needs
turning, overlaps rejected. Tour: one deliberate placement on a taken cell
(rejected), then two boxes packed by clicking.
