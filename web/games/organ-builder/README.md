# Orgelbauer

Stand pipes in a little organ so it plays a tune left to right. Pipe length follows
pitch (length proportional to 1/frequency, C4 = 2.6 m), and the voice is a small
additive flue-pipe synth with a breathy chiff (WebAudio, only after a gesture).
Tunes, all traditional or long out of copyright: Frère Jacques, Alle meine
Entchen, Ah vous dirai-je Maman, Ode an die Freude, Hänschen klein.

Self-test: the bench always holds exactly the tune's notes and is never presolved,
longer pipes are always lower, pipes fit the case, a lucky order has under 1% odds
(worst 0.48%), and the checker rejects a swap. Tour: plays a version with two notes
swapped (must fail, red tags), fixes it, then plays the next tune.
