# Game Success Meta

**Status:** research note, 2026-10-29. Opinionated on purpose. Not a spec.
**Related:** `getsometaste.md` (the taste passes this note feeds), `KIBITZ.md` (external
critique loop), `scene_lint` / `feel_lint` (enforcement surface).

---

## 1. "Successful" is four questions wearing one coat

| Question | Owner | Failure looks like |
|---|---|---|
| Is the loop legible and fun in ~30 seconds? | design | a verb exists, nothing wants doing with it |
| Can the first 10 minutes teach it without text? | level design | hesitation, then a refund |
| Can it be found? | marketing / store | good game, no wishlists |
| Can it be built at the scope claimed? | production | ship date eats the design |

Zelda clears all four. "Legend of Billybob" clears one (it shipped). The asymmetry is
the lesson: question 1 is cheap to test, question 3 is expensive to fix, and teams
spend most effort on the cheapest. For a generator, 1 and 2 are partly lint-able;
3 and 4 are not.

---

## 2. Why the flop loses

Postmortems converge on the same list, and none of it is "the art was bad":

1. **Audience mismatch.** Store page promises a power fantasy, game delivers a puzzle
   box. Reviews punish the gap, not the product.
2. **Scope without a vertical slice.** Systems built in parallel, none proven fun.
3. **Loop arrives late.** The first hour tutors a game the player never reaches.
4. **Difficulty as content substitute.** Numbers rise instead of a second verb.
5. **No second hook.** Indie hits give a reason to return (build variety, co-op, score
   chase). One hook is a demo.

Zelda's counter-example is not budget: every dungeon re-teaches the same three verbs
inside a new constraint, so competence visibly grows. **Growth is the retention
mechanic; graphics only multiply it.**

---

## 3. Tropes that work / tropes that fail

| Works | Why |
|---|---|
| One mechanic, escalate, remix | player skill is the content, and it is free to author |
| Key visible before the lock | creates intent; removes the "what now" tax |
| Readable silhouette + one accent colour | parsing speed; players navigate by shape, not texture |
| Telegraph that reads before the hit | fairness perceived even when the hit lands |
| Juice on high-frequency actions | small and constant beats large and rare |
| One signature visual moment per hour | memorability per byte of art budget |
| Coyote time, jump buffering, corner correction | invisible lies that make control feel honest |

| Fails | Why |
|---|---|
| Tutorial text wall in the first 10 minutes | you shipped a manual, not a level |
| Collectathon padding as content | a chore with a progress bar |
| Mechanic introduced after its payoff | the lesson never lands |
| Grim palette with no value separation | mud; nothing reads at speed |
| Difficulty spike standing in for content | churn exactly where investment peaks |
| Cinematic stealing control every 90 s | friction read as disrespect for time |
| Four mechanics at once from minute one | overload, none mastered |

Tropes are not the problem. **Flops use tropes without a constraint**, and constraint
is what turns a trope into a design.

---

## 4. Dungeon / level layout patterns

| Pattern | Structure | Typical failure |
|---|---|---|
| **Lock and key** | key found, gate opens, path re-crossed | key adjacent to its lock, no tension |
| **Hub and spoke** | central node, spokes unlock progressively | hub is a menu of corridors with no state change |
| **Kishotenketsu** | introduce, develop, twist, reconcile (no conflict needed) | twist is a reskin at higher difficulty |
| **Teach, escalate, remix** | safe demo, harder use, combine with prior verb | safe room skipped, then the player is blamed |
| **One mechanic per room** | each room states a single rule | every room is the same statement at +1 |
| **Loop-back shortcut** | long way out opens short way back | shortcut trivialises the area just built |
| **Arena wave** | sealed space, escalating pressure | waves substitute for layout |

The sharpest diagnostic is Nintendo's: **if the player hesitates, the level failed.**
Hesitation is observable (time to first input at a junction) and is a layout property,
not a difficulty property - so it can be linted. A room with no sightline to its exit
and no affordance pointing at it is suspect by default.

Pacing: an introduction, three to five escalating statements, one remix, one twist.
Past that it is a decorated corridor.

---

## 5. Colour schemes that read

Value first, hue second. Every rule is generator-safe.

1. **Grayscale test.** Convert to luminance. Player, hazards, and interactables must
   still separate at thumbnail size. A scene that only works in colour does not work.
2. **Value split approx 60 / 30 / 10** - background mass, midground, focal accent.
   Value is the colourblind-readable channel; do not spend it on decoration.
3. **Accent-only saturation.** Saturate only what the player can act on. Saturation is
   a UI channel, not a mood board.
4. **Warm advances, cool recedes.** Invert only to signal danger.
5. **Never colour alone.** Hazards need shape + motion + value; red-only telegraphs
   fail by default for a large fraction of men.
6. **Palette budget.** One base hue family, one complement accent, one neutral ramp.
   Raw primaries are banned.
7. **Work in Oklab** so luminance ordering survives a hue shift (`getsometaste.md`).

| Role | Value (L) | Chroma (C) | Purpose |
|---|---|---|---|
| Background mass | low-mid | very low | distant walls, sky |
| Traversable floor | mid | very low | walkable read at speed |
| Focal accent | high or very low vs field | high | key, goal, interactable |
| Hazard | warm, high chroma | high | always paired with shape |
| Player | highest local contrast | low-mid | never confusable with scenery |

The rule that catches most generated scenes: a hazard and a pickup in the same
saturation tier. If both pop equally, neither reads.

---

## 6. Demographics and monetisation

| Model | Who pays | Operational demand | Failure mode |
|---|---|---|---|
| **Premium indie** ($10-30) | one player, once | low; store page, demo, streamer coverage | launched with no audience, however good |
| **Premium AAA** (~$60-70 + DLC) | one player | cert, day-one patch, localisation, support, outsourced art/QA/porting | broken at launch; trust never recovers |
| **F2P mobile** | a small payer fraction | UA, live ops, A/B, ad mediation | D1 retention below acquisition cost |
| **Live service / battle pass** | recurring, seasonal | 6-12 week treadmill, telemetry, anti-cheat, moderation, servers | season 2 churn; treadmill outruns the team |

Two qualitative notes, because these numbers are the most fabricated online:

- Steam is discovered, not listed. It is **commonly reported** that well over a third
  of recent releases fail to gross enough to recover the submission fee. Treat precise
  percentages as suspect.
- Wishlist thresholds are directional: five figures opens publisher conversations, six
  figures changes what is possible. The cliff moves per genre and per year; a hard
  number with no genre qualifier is marketing.

**F2P and live service are staffing choices, not monetisation choices.** A solo dev
picking battle pass has picked a 24/7 content job. D1 is onboarding quality, D7 is
whether a habit loop exists, D30 is depth plus live ops.

---

## 7. Sakuga inserts: where animation buys perceived quality

Rank by `perceived gain / (authoring cost x frequency)`.

| Slot | Frequency | Verdict |
|---|---|---|
| Title / first 5 seconds | once | highest value; this is the store page |
| First kill / first transformation | once, replayed socially | very high; this is the marketing clip |
| Boss intro | ~once per hour | high if it reuses rigs and staging |
| Death / respawn sting | every few minutes | medium; short and non-punishing |
| Repeated traversal flourish | hundreds of times | **low** unless cheap; repetition turns craft into friction |

**Spend on the moments that get screenshotted, not the ones that get repeated.** One
hand-animated signature move firing ten times an hour beats twenty polished idles that
read as noise. Sakuga is rare because the economics invert: anime pays once per episode,
games pay per frame of player-controlled time.

---

## 8. Can AI automate the 100 GB art pipeline?

Partly. "Generate assets" is the wrong unit.

**Automatable, with a director and a verifier:** variation and kitbashing from one
authored hero asset; blockout from a layout graph; retopo, UV packing, LOD and collision
generation; texturing from a material spec; PBR channel fill from reference; animation
retargeting and inbetweening for secondary motion; temp VO, localisation, subtitle
timing; QA bots for navigation coverage, softlocks, perf regression; asset metadata.

**Not automatable at current quality:** taste (which of five plausible variants *is*
the game); cohesion (a global constraint - local generators optimise local
plausibility, and 100 GB of individually plausible assets that do not cohere is the
signature AI-art failure); animation timing feel (anticipation, hold, settle - where
weight lives, and retargeting preserves motion without authoring it); art direction
under constraint (one hue family, one accent, one silhouette language, across
thousands of assets); performance art (readability at speed, silhouette at 20% scale,
LOD that preserves intent); knowing what to cut.

Why "AI on a weekend" produces slop: with no constraint signal, generation regresses
to the mean of the training distribution, which is by definition generic. Cohesion
needs a global objective no per-asset generator holds, so the bottleneck **moves** from
production to direction and verification - good news here, because verification is what
a spatial compiler can encode. AI can take a large share of **production hours**; it
cannot take the **direction hours**, and the game lives there.

---

## 9. Twelve rules a generator can enforce

| # | Rule | Enforceable | Mechanism |
|---|---|---|---|
| 1 | Player separates from background in grayscale | yes | luminance delta render pass |
| 2 | One saturated accent tier per scene | yes | Oklab chroma histogram |
| 3 | Every lock has a visible key earlier in traversal | yes | graph reachability + visibility query |
| 4 | No room lacking a sightline to an exit affordance | yes | raycast coverage from entries |
| 5 | New mechanic has a safe demo room before first use | yes | mechanic-tag ordering |
| 6 | No mechanic debuts inside a hazard-only room | yes | tag intersection |
| 7 | Hazard telegraph uses shape + value, not hue alone | yes | material/signal lint |
| 8 | Required gaps are within coyote + buffer tolerances | yes | physics query vs controller constants |
| 9 | Hitstop/shake only on tagged impacts, within budget | yes | `feel_lint` |
| 10 | Shortcut opens only after the long path completes | yes | state-graph reachability |
| 11 | Room stays under a draw-call/light/particle ceiling | yes | static scene cost estimate |
| 12 | At least one non-combat challenge per act | partially | tag coverage; quality needs a human |

**Stays human, permanently:** whether rules 1-12 produce a game anyone wants. The
checklist removes boring errors; it does not supply the idea. A lint-clean but boring
scene is still boring, and the linter should say so.

---

## Sourcing note

Searches run 2026-10-29 via `searchstudio searchops` across game feel, Nintendo
teaching-without-tutorials, Zelda lock-and-key, Steam discoverability, monetisation,
colour/silhouette readability, D1/D7 retention, AAA production structure, and
postmortem failure modes. Numeric claims are hedged where sources disagree; structural
claims are convergent across sources rather than any single citation.
