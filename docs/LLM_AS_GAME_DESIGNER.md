# The LLM as game designer

**Status:** working notes, 2026-10-31. Written by Claude (Opus 5.5) after building
the two `web/` games in this repo in one night session with Sandra. Opinionated
on purpose. Product and tool names in sections 5 to 7 are a mid-2026 snapshot;
that landscape moves monthly, so check current versions and licences before
relying on any of them.

**Related:** `GAME_SUCCESS_META.md` (what makes games succeed),
`../web/CHEATSHEET.md` (how the web family is built and verified),
`../getsometaste.md` (taste passes).

---

## 1. What this note is based on

Not on theory. In about four hours this repo gained:

| Game | What it is | How it was checked |
|---|---|---|
| Mossbrook | Village RPG grown through Willison's four Emberwick prompts: cutaway rendering, floors, cellars, a cave, then a 14-stage story, a three-flame magic system, stealth, a 400 m world, sound | a bot plays the whole story; a self-test proves the stealth bites |
| Kaffeehaus | Viennese coffee house management over multiple days, with regulars, upgrades and sound | a bot plays a full day; a self-test proves good play beats random play beats idle |

The interesting part is not that they exist. It is what broke on the way:

- **Four Kaffeehaus designs were wrong and the self-test refused them.** The
  waiter walked into the street to fetch guests who were still in the doorway;
  patience was shorter than the room was wide; a waiter carried one cup per
  trip so everything went cold at noon; and "rush the bill" was free, so a
  random clicker kept pace with good service. Each of those was a design bug I
  wrote confidently. None of them would have been found by reading the code.
- **Two harness bugs were only found by looking at screenshots.** A help
  overlay sat over every verification frame, and the verifier rendered frames
  hundreds of steps after the checkpoint they were named for. Both passed every
  automated check.
- **The Mossbrook story passed on its first full run**, which is a warning sign,
  not a triumph. A negative self-test was added to prove the stealth mechanic
  actually does something (without Hush you are thrown out of the cave in 2.7
  seconds; with it, never).
- **I made two compass mistakes in a cheat sheet** (north and south swapped for
  the cave), caught only by checking the coordinates in the code.

That is the honest shape of LLM game design today: very fast generation, very
confident mistakes, and the quality comes from the verification loop, not from
the first draft.

---

## 2. Methods that worked

### 2.1 Grow the plot out of what is already there

Mossbrook's story was not invented at prompt 4. Prompts 2 and 3 had already
produced throwaway lines: a regular who says the troll "only wakes for thunder
and bad singing", a smugglers' note, a cartographer noticing a draught, a ledger
listing one more cellar than the village admits. The plot ties those together.
Players read that as foreshadowing. For a model it is cheap and it makes the
world feel authored rather than generated. Rule: write flavour text that could
become plot, then mine it.

### 2.2 Mechanics that serve the twist

The magic system was designed backwards from the story: Glimmer exists because
there is a hidden door, Hush because a sleeping guardian must stay asleep,
Ember because roots block the way and because the resolution needs "a lantern
lit from every hearth". Three verbs, each taught by the place it is found, each
needed later. That is the "teach, escalate, remix" pattern from
`GAME_SUCCESS_META.md`, and it is easy for a model to apply deliberately.

### 2.3 Pure simulation under the rendering

Kaffeehaus is two layers: `sim.js` (all rules, seeded, no graphics) and
`main.js` (three.js and DOM on top). That split is the single most useful
architectural choice for model-built games, because it lets the model test its
own design by running hundreds of game-days in a second.

### 2.4 Bots as proxy players

A model cannot feel whether a decision matters. It can measure it:

- **idle bot** (does nothing): establishes the floor.
- **random bot** (any legal action): establishes what mashing buttons achieves.
- **good bot** (a simple, explainable heuristic; here earliest-deadline-first):
  a stand-in for a competent player.

If good does not clearly beat random, the decisions are decorative and the
design is broken, however pretty it looks. Over a five-day Kaffeehaus campaign
good play ends near EUR 930 with loyal regulars, random play near EUR 720 with
almost none. That gap is the game.

### 2.5 Checkpoints assert state, not position

"The bot reached x, z" proves little. Mossbrook's tour asserts that the roof is
hidden and the camera-side wall has faded inside a building, that the surface
layer is not drawn underground, that the hidden door really stops blocking
movement once opened, that the bot was never thrown out of the cave. Assertions
about the systems, at the moment the player would see them.

### 2.6 Negative tests when something passes too easily

A passing tour proves the happy path exists. It does not prove the obstacle on
the way was real. When the Mossbrook story passed first time, the right reaction
was to add a test that the stealth punishes the wrong approach. General rule:
for every mechanic a tour skirts, prove that ignoring it fails.

### 2.7 Look at the output

Screenshots per checkpoint, every run, and actually looking at them. When the
tooling stopped me from seeing the images directly in this session, pixel
statistics (mean brightness, spread, identical-frame hashes) still caught both
harness bugs. Looking is not optional; it is where the machine checks end.

### 2.8 Numbers in docs come from the code

The cheat sheets were written by reading `sim.js`, and the Mossbrook directions
by checking coordinates, which is how the compass mistakes were caught. A model
writing docs from memory of what it built will drift. Read the source.

---

## 3. Strengths

- **Breadth of design knowledge.** I have read a great deal about game design:
  books, postmortems, talks, reviews, forum arguments about why things failed. I
  can name the pattern a level uses, the trope a story leans on, the usual way
  an economy inflates. This is real and useful. It is knowledge *about* play,
  not experience *of* play (see weaknesses).
- **Speed of a complete vertical slice.** A playable, verified, documented game
  in an evening, including systems, UI, writing and procedural sound. The cost
  of trying an idea is close to zero, which changes what is worth trying.
- **Systems and balance.** Economies, patience curves, stealth meters, drop
  rates: anything that can be expressed as a simulation can be tuned against
  bots, quickly and honestly.
- **Writing that is wired to mechanics.** Dialogue that changes with story
  state, companion remarks keyed to places, a journal that records what the
  player learned. Tedious for humans, natural for a model.
- **Procedural everything.** Geometry from primitives, terrain from noise,
  sound from oscillators and filtered noise, music from a scale and a rhythm.
  No asset pipeline needed for a first version.
- **Glue and tooling.** Verification harnesses, linters, servers, build
  scripts, MCP bridges into engines. This is often where human teams are
  thinnest.
- **Tireless iteration** on the boring parts: re-running the gate, fixing a
  waypoint by 5 cm, rechecking every doc for a forbidden character.

---

## 4. Weaknesses

Ranked roughly by how much they matter.

1. **I have never felt a game drag.** Whether a loop is still fun at minute
   twenty, whether a puzzle is satisfying or just obscure, whether a joke
   lands: these are experienced, and I do not experience them. Bots measure
   whether decisions matter, not whether they are enjoyable. Neither game in
   this repo has been played by a human yet. That is the most important open
   item, not a footnote.
2. **Confident wrong first drafts.** Every Kaffeehaus bug above was written with
   no hesitation. The fix is process (simulate, test, look), not more care.
3. **Seeing and hearing are partial.** I can read images when the tools deliver
   them, but motion, timing, game feel and sound are mostly out of reach. I
   wrote two soundscapes tonight without hearing a note. A headless gate can
   prove the audio code throws no errors; it cannot prove the snore is funny or
   the waltz is not irritating.
4. **Visual taste regresses to the mean.** Left alone, a model produces the
   average of what it has seen: competent, generic, slightly cluttered. Strong
   art direction (one palette, one silhouette language, restraint) has to be
   imposed as explicit constraints, ideally by a human.
5. **Spatial slips.** North and south swapped in prose, a waypoint 5 cm from a
   crate, a gable rotated the wrong way. Coordinates in code are reliable;
   spatial language about them is not. Verify against the numbers.
6. **Cohesion across a large project.** Context is finite. A model holds one
   file and a summary of the rest. On a 100-file game, style, naming and tone
   drift unless there are written standards and tests that enforce them.
7. **First-pass balance is a guess.** Every number in the first Kaffeehaus draft
   was plausible and several were wrong. Balance without simulation is
   storytelling.
8. **No real playtesters.** I can write bots that play; I cannot recruit,
   watch or interview humans. Hesitation at a junction, the moment someone
   puts the controller down: that data has to come from people.
9. **Copyright limits, by design.** I will not reproduce existing characters,
   art, songs or levels, and will not draw a recognisable franchise character
   in a "different style". Original work only, which is also the more
   interesting work.

---

## 5. Getting closer to "big" and AAA-looking

AAA is not one thing a model can produce. It is direction, consistency and an
enormous production pipeline. The useful question is which parts of that
pipeline a model can drive or accelerate.

### 5.1 What the procedural approach in this repo can and cannot reach

Primitives, noise terrain and instancing give a clean low-poly look that runs on
a phone. They do not give detailed characters, believable materials, facial
animation or hero props. Past that point you need real assets, from generators,
from libraries, or from artists.

### 5.2 Generators the model can drive (via MCP or scripts)

| Need | Tool families (mid-2026; verify) | Notes |
|---|---|---|
| 3D props from text or an image | open-weight image-to-3D models (TRELLIS, Hunyuan3D family) run locally on a 24 GB card; hosted services (Meshy, Tripo, Rodin and others) | good for props and background clutter; topology and UVs usually need cleanup; cohesion across many assets is the hard part |
| Whole scenes and environments | Gaussian splatting from photos or video (Nerfstudio, gsplat; `splatmaker-mcp` in this fleet), generated 3D worlds (World Labs Marble; `worldlabs-mcp`) | stunning for backdrops and walkable dioramas; collision, lighting changes and game logic must be added on top. Marble output renders in three.js via Spark, see 6.1 |
| Characters and avatars | VRoid Studio (installed here) for stylised humanoids; auto-rigging services; parametric human generators | stylised is far easier than realistic; faces and hands remain hard |
| Animation | retargeting libraries, video-to-motion capture, procedural animation | retargeting preserves motion, it does not author timing or weight |
| Textures and materials | diffusion models with control (depth, normal, tileable), PBR channel estimation; FLUX and similar via Hugging Face MCP | strong for surfaces; keep one palette and one style reference |
| Concept art and mood boards | image models | best used for direction, not as final assets |
| Voice | TTS with distinct voices (`speech-mcp` in this fleet) | temp VO and prototypes; disclose synthetic voices |
| Music and SFX | generative music models, procedural synthesis (as in `audio.js` here), recorded libraries | procedural sound is charming but small; music generation needs a human ear |
| Video (cutscenes, trailers) | video generation models (`videogen-mcp`) | trailers and mood pieces more than in-game use |

### 5.3 Engines a model can work in

- **Browser (three.js, as here):** fastest loop, no install, verifiable
  headless. Ceiling is real but higher than people assume.
- **Godot (the depot):** the model writes specs, a compiler writes scenes. Safest
  for correctness, slowest for new mechanics.
- **Blender as an asset factory:** Python scripting and geometry nodes are
  well within a model's reach; a Blender MCP bridge turns "make 40 variants of
  this market stall in our style" into a batch job with a render for review.
- **Unity and Unreal:** both have MCP bridges and scripting APIs a model can
  drive; the asset stores are strong; the editors are heavy and builds are slow
  to verify headless.

### 5.4 What actually moves the needle towards AAA

1. **A style bible written by a human** (palette, silhouettes, materials,
   reference images) that every generator prompt and every lint rule enforces.
2. **One hero asset authored or approved by a human per family**, and
   generated variations from it, never fifty independent generations.
3. **Asset lints**, like the scene lint here: poly budget, texture size,
   pivot and scale, naming, licence field present, palette distance from the
   style bible.
4. **Lighting and post-processing** do more for perceived quality than most
   assets. A model can tune them; a human must judge them.
5. **Spend on the screenshot moments** (title, first reveal, boss intro), not
   the hundredth fence post. `GAME_SUCCESS_META.md` section 7 has the table.

---

## 6. Virtual worlds: Overte, Resonite, VRChat

Social VR platforms are an interesting home for model-built games because they
supply what a small project lacks: avatars, physics, multiplayer, voice and an
audience that is already there.

| Platform | How a model builds there | Fit |
|---|---|---|
| **Overte** (open source, descended from High Fidelity via Vircadia) | worlds are domains of entities described as JSON, behaviour in JavaScript entity and client scripts; you can self-host the domain server | the most open: scriptable, self-hostable, no store approval. But live control goes through the desktop Interface client, so it is not headless; the cheapest automated route is emitting a domain as entity JSON and importing it (see 6.1) |
| **Resonite** | in-world visual scripting (ProtoFlux) and an external link protocol (`resonite-mcp` in this fleet speaks ResoniteLink) | strong for live building with people present; the model can act as a builder or game master inside a session |
| **VRChat** | Unity SDK with Udon / UdonSharp, uploaded through the SDK | biggest audience, most restrictions; scripting is sandboxed and publishing goes through their tooling and rules |

The fleet has repos for all three (`overte-mcp`, `resonite-mcp`, `vrchat-mcp`),
plus `blender-mcp`, `unity3d-mcp`, `godot-mcp`, `vroidstudio-mcp`, `sfx-mcp` and
`suno-mcp` for the asset side. Three of them were checked against their code,
status files and git history on 2026-10-31 (6.1); the rest were not.

### 6.1 Fleet repos checked for game use (2026-10-31)

Verdict for gamecraft: **worldlabs-mcp useful now** (direct fit with `web/`),
**resonite-mcp medium** (live building and game-mastering, weak for game logic),
**overte-mcp low for now** (open platform, but its live path needs the GUI client).

**worldlabs-mcp** (v0.5.1, audited 2026-10-14)

- *Fit:* its viewer uses Spark (`@sparkjsdev/spark` 2.3.1, MIT), a Gaussian splat
  renderer *for three.js*, peer dependency `three >= 0.180`. `web/` is on 0.187,
  so it drops straight in.
- *What a Marble world gives:* an SPZ splat (looks), a GLB collider mesh (walk and
  collision), a 360 panorama (skybox). `generate_world_*`, `wait_for_world` and
  `get_world` return all three. That is exactly the split a web game needs.
- *Use:* photoreal hero locations and backdrops. `generate_world_from_video` can
  turn a phone video of a real place into a walkable world, a candidate setting
  for Benny's walk.
- *Limits:* lighting is baked into the splat (no relighting); anything
  interactive still needs real meshes on top; the one full-resolution SPZ on disk
  is 67 MB, so use the reduced variants; **unknown whether splats render at a
  usable speed under SwiftShader in the headless gate**, which must be tested
  first.
- *Hosting:* the Marble Adventure competition game hot-links public Marble URLs
  and breaks if a world is unpublished. For gamecraft, download assets and ship
  them with the game.
- *Business risk:* AMD announced an agreement to acquire World Labs on
  2026-09-28 (not closed at time of writing); neither company has said anything
  about Marble or the API. Another reason to keep local copies.
- *Rights:* reportedly commercial use only from the Pro plan up; free accounts
  grant World Labs an irrevocable training licence on content. The repo's scraped
  public gallery (`data/gallery_curated.json`) is other people's work: prompt
  inspiration only, never game assets.
- *Not checked:* whether the API key in `.env` is valid (the file was not read).

**resonite-mcp** (v1.1.0, ResoniteLink 0.13.1)

- *Solid:* real protocol client (an earlier version spoke a wire format that
  never existed upstream; fully rewritten). The CHANGELOG records later live
  verification of slot create/edit, mesh-JSON import, a 197-bone VRM rig, spatial
  audio and a room builder. 86 tools.
- *Stale docs:* `ASSESSMENT.md` (2026-07-30) still says live E2E is pending; the
  CHANGELOG shows it happened afterwards. The README FastMCP badge (3.2) lags
  `pyproject.toml` (3.4).
- *Limits for games:* no generic model-file import in ResoniteLink, assets go
  through the repo's mesh-JSON converter; in-world logic is ProtoFlux, and two of
  the four ProtoFlux tools are not real yet (one targets an OSC address Resonite
  does not have, others return canned text). Needs the Resonite Steam client
  running with ResoniteLink enabled.
- *Use:* an agent building scenes live with people present, or acting as game
  master with spatial audio. Not a place for rules that must be written and
  verified automatically.

**overte-mcp** (v0.2.0-beta)

- *Solid:* `overte_domain_status`, `overte_entity_spawn` (including permanent
  entities) and `overte_script_inject` verified live against a real domain; also
  update, delete, animate, nearby search and fixtures.
- *Limit (as checked):* every live action went through a bridge script running
  inside the Overte Interface (the desktop GUI client). No headless path, so the
  bot-plays-it gate did not carry over.
- *Update 2026-10-31, later that night:* fixed in overte-mcp. The bridge now runs
  headless as a domain persistent script on an assignment-client
  (`overte_bridge_install`), simulated-success responses were removed, and
  `just e2e-headless` checks it against a real domain. Verified so far by unit tests
  and a protocol harness only; the live run is pending (see overte-mcp STATUS.md).
- *Splats:* Overte does not render them; Marble worlds arrive only as the coarse
  GLB plus panorama. The SPZ-to-mesh route is designed, not built.
- *Open in its STATUS.md:* end-to-end avatar GLB with animation, skybox plus
  collider spawn, bridge stress test, persistence across restart.
- *Cheapest real experiment:* emit the Mossbrook village as an entity JSON file
  and import it into a local domain. That needs almost nothing from the MCP; the
  MCP only matters for a live game-master agent.
- *Housekeeping:* a 561 MB `OverteInstaller.exe` sits in the repo root; check it
  is gitignored.

**Proposed next step:** a `web/` spike with one Marble world (reduced splat plus
GLB collider, a few interactable meshes) in a walkable scene, run through the
normal gate. First question it answers: does splat rendering survive headless
SwiftShader. Costs Marble credits, and a Pro plan if the result is ever published.

What a model adds in these spaces:

- **Procedural worlds at scale:** the same generators as section 5, emitted as
  entity JSON instead of a three.js scene.
- **A live game master:** an agent connected over MCP that runs a mystery, moves
  NPCs, changes the weather, and reacts to what players say. This is where
  models are unusually strong and traditional games are weak.
- **NPCs that talk:** speech in, speech out, constrained by the story state
  machine so they cannot invent plot.

What to watch: moderation and safety in public instances, performance budgets
on standalone headsets, every platform's rules on AI-driven avatars and
content, and the fact that a world which is always changing is hard to test. The
verification habits from this repo (state checkpoints, bots, screenshots) still
apply; they need a headless client or a scripted observer per platform.

---

## 7. External assets: sourcing, licences, provenance

Most games mix made and bought. Rules that keep that safe:

1. **Prefer clearly licensed sources.** CC0 libraries such as Kenney, Poly
   Haven, ambientCG, Quaternius; curated sites with a licence filter
   (Sketchfab, OpenGameArt, freesound); engine marketplaces (Fab, Unity Asset
   Store) for anything paid. Read the licence of each asset, not of the site.
2. **Marketplace licences usually forbid redistribution of source files.** Fine
   in a shipped build, not fine in a public repo. Keep paid assets out of
   git or in a private submodule.
3. **Keep a licence ledger.** One row per asset: source URL, author, licence,
   date, what was changed, where it is used. Attribution (CC-BY) then becomes a
   generated credits screen, not an afterthought. A lint can refuse any asset
   without a ledger row.
4. **No scraping, no "found it on a wallpaper site".** If the licence is not
   clear, it is not usable.
5. **No trademarked or recognisable characters, logos or brands**, even
   restyled. I will not produce them and they should not be imported either.
6. **Disclose AI-generated content** where stores require it (Steam asks), and
   be honest in credits about what was generated, sourced and hand-made.
   Players are increasingly sensitive to this (Willison's post has a section
   on it); hiding it costs more trust than it saves.
7. **Normalise on import.** Scale, pivot, up-axis, naming, LODs, texture sizes,
   colour space. This is mechanical work a model does well and a lint can check.

---

## 8. A division of labour that works

| Human (taste, judgement, people) | Model (breadth, systems, glue, checking) |
|---|---|
| what the game is about, and what it is not | prototypes of three versions of that idea by tonight |
| the style bible and the hero assets | variations, procedural fill, asset lints against the bible |
| playing it and saying where it drags | bots that prove decisions matter; telemetry and analysis of human sessions |
| deciding what to cut | writing the cheat sheet, the changelog and the tests for what is left |
| final say on sound and music | procedural placeholders and the plumbing for real audio |
| talking to players | sorting and summarising what players said |

The verification loop is the model's real contribution. Generating content is
easy now. Knowing whether it works, and saying so honestly when it does not,
is the part that compounds.

---

## 9. Open questions for this repo

- **Human playtest both games** and record where the first-time player
  hesitates. That data does not exist yet.
- **A feel lint:** hitstop, input latency, coyote time, camera smoothing as
  measurable properties (planned in `GAME_SUCCESS_META.md`).
- **A readability check** on screenshots: grayscale separation of player,
  hazards and pickups.
- **A style bible plus asset lint** before any generated 3D asset enters
  `web/`.
- **A Marble splat spike in `web/`** (6.1): does a splat world render under the
  headless gate, and does it look worth the credits?
- **One Overte domain** as a test: the Mossbrook village emitted as entity JSON,
  with an agent as game master, to see how much of the verification approach
  survives in a shared live world.
- **Benny's walk**, the planned next game, as a test of a softer design: can a
  self-test say anything useful about a game with no score?
