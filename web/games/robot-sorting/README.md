# Boomy Sorts

A Raspbot-like robot at a conveyor junction sorts parcels (4 colours x 2 sizes x
star or not = 16 kinds) into LEFT, RIGHT or ON using up to four first-match rules:
IF a [AND b] THEN bin. Five jobs, the later ones needing the right rule order.
The shift is the same parcels every run, so changes are comparable; wrong ones get
a red flag.

Self-test (5 jobs x 15 shifts): reference programs fit the slots and sort all 16
kinds, an empty program never passes, wrong-order programs fail on every shift, and
every outcome appears at least twice per shift. Tour: drives the real selects and
Run button: job 1 empty (fails), job 1 fixed, job 2, job 3 in the wrong order
(fails), job 3 right.
