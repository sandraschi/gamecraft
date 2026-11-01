# Kage-e

Six pictures (Mount Fuji, a house, a cat, a teapot, a rocket, a familiar face) drawn
as ASCII masks. Each becomes a sculpture: a greedy rectangle cover of the mask, every
block at its own random depth. A shadow-mapped directional light projects it on the
wall; from the front the shadow is exactly the picture, from anywhere else a jumble.

Solved is measured as silhouette overlap (IoU of projected block hulls vs the mask),
so a mirror pose counts for symmetric pictures. At 70% overlap, letting go lets it
drift home; at 90% it settles exactly.

Self-test (6 pictures x 6 seeds): exact pose overlaps 99.9%+, starts below 62%,
side view far from solved, asymmetric mirrors never solve, a hand-reachable 3 deg
off lands in the assist basin. Tour: a wrong drag first, then Fuji and the house.
