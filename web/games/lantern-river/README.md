# Lantern River

Toro nagashi at night: paper lanterns drift down a meandering river past rocks.
Hold the pointer on the water and lanterns within 2.6 m drift away from it. A cove
under a torii on the far bank catches any lantern that reaches it; every two light
a stone lantern, twelve light the gate. Misses float on to the sea. No fail state.

`sim.js` is pure. Self-test (5 seeds x ~30 lanterns per policy): the current alone
brings 10% or fewer home (measured 0%), random breathing under 50% (0%), careful
breathing 80% or more (99%), and every lantern is accounted for.
Tour: watches the first lantern drift past untouched, then breathes through the
screen on the river side of each lantern until the gate is lit.
