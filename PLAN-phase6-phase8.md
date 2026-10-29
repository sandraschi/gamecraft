# PLAN Phases 6–8: Taste, Universal Harness, and the Monetization Paradox

**Status:** DRAFT / QUEUED (2026-10-29).  
**Fence:** **Do NOT start until Phase 5 depot is `[STATUS: SUCCESS]`.**  
**Owner:** Builder chat (OpenCode). Reviewer/Strategy: Antigravity / Sandra.

---

## Executive Overview

Where Phase 5 proves **mechanical generality** (12 playable Godot 4.3 archetypes with 0 lint errors and verified 600-frame win traces), Phases 6 through 8 elevate GameCraft from an engineering verifier into a commercial, aesthetic, and economic compiler:

- **Phase 6:** The Taste & Juice Compiler (Injecting the `getsometaste.md` cookbook).
- **Phase 7:** The Universal Backend & Steam/Platform Harness (Engine-agnostic IR, Bevy/Native binary, Steam Deck compliance).
- **Phase 8:** The Economy Engine & The Sludge Paradox (Monetization state machines, Steam Trading Cards/DLC, and solving the post-abundance discovery meta-game).

---

## Phase 6: Taste, Juice & Aesthetic Compiler (`gamecraft-juice`)

**Goal:** Turn mechanically sterile games into tactile, visually cohesive indie titles in an afternoon by applying deterministic aesthetic middleware.

### 1. Pass A: Tactile Mechanics (`feel_lint`)
Inject forgiving physics lies into character GDScript templates:
- **Coyote Time:** 4–6 frame jump allowance post-ledge departure.
- **Jump Buffering:** 5-frame input pre-cache before grounding.
- **Apex Float:** Halve gravity within $|v_y| < 1.5\,\text{m/s}$ for 3 frames to give responsive air control.
- **Corner Correction:** Raycast nudge rising bodies around 90° ceiling corners.
- **Variable Jump Cut:** Early jump release clamps upward velocity by $0.5\times$.

### 2. Pass B: Juice Middleware
Add procedural visceral feedback nodes to the scene root:
- **Trauma Screen Shake:** $\text{Offset} = \text{Trauma}^2 \times \text{Noise}(t)$, decaying linearly.
- **Hit-Stop (Micro-Freeze):** Set `Engine.time_scale = 0.0` for 2–4 frames on combat strikes or brick impacts.
- **Squash & Stretch Springs:** Deform visual meshes on acceleration/impact, recovering over 100ms.
- **Particle Poofs:** Dust bursts on landing; sparks on impacts; confetti on victory triggers.

### 3. Pass C: Visual Harmony & Color Constraints (*Old Man's Journey* / *Gris*)
- **Oklab Harmonic Material Decks:** Restrict palette choices to curated color spaces (analogous terracotta/sage nostalgia, pastel triadic dreamscapes). Ban raw primary RGB.
- **Atmospheric Depth:** 3 parallax background layers ($0.2\times, 0.5\times, 0.8\times$) with horizon distance tinting.
- **Cinematic Post-Processing:** 2.39:1 letterbox framing and subtle vignette shading.

### 4. Pass D: Audio Aesthetics
- **Pentatonic Quantization:** Restrict procedural SFX and musical chimes to pentatonic scales to eliminate dissonance.
- **Micro-Pitch Jitter:** Modulate repeating SFX by $\pm 5\%$ pitch.

---

## Phase 7: The Universal Backend & Steam Platform Harness

**Goal:** Graduate from Godot-specific text generation to an engine-agnostic Scene IR that compiles to native binaries and commercial platforms.

### 1. Decoupled Scene IR Architecture
- `gamecraft-core` functions as the **LLVM of Game Worlds**.
- Backends as interchangeable targets:
  - `gamecraft-godot`: Godot 4.x `.tscn`/`.tres` export (editor inspection, easy modding).
  - `gamecraft-bevy`: Pure-Rust native binary (single statically linked `.exe`, zero runtime overhead, instant boot).
  - `gamecraft-web`: WebAssembly target for instant web browser / itch.io embeds.

### 2. Commercial Platform Packaging (`gamecraft-pack`)
- **Save State Serialization:** Utilize `gamecraft-commands` JSON logs for zero-effort Steam Cloud sync.
- **Steam Deck Verification Standard (`steam_deck_lint`):**
  - Native 1280×800 (16:10) viewport scaling without stretching or black clipping.
  - Gamepad-first input mapping with dynamic button prompts (Deck, Xbox, DualSense).
  - Minimum 12pt font legibility at 720p/800p handheld distance.
  - Clean sleep/suspend recovery without memory leaks or audio buzzing.
- **Headless Build & Deploy:** `just export-steam` bundling standalone binaries with icons and manifests ready for automated `steamcmd` deployment.

---

## Phase 8: Economy, Monetization & The Sludge Paradox

**Goal:** Model game monetization as a deterministic state machine while navigating the market realities of infinite, zero-cost game production.

### 1. The Monetization Mechanics
- **The Honest Steam Indie Model:**
  - Automated generation of the "Supporter Edition" bundle (Base game + FLAC soundtrack + 3 4K wallpapers + exclusive cosmetic skin).
  - Steam Community asset synthesis: 5 trading cards, 5 emoticons, 3 profile backgrounds, and badge definitions.
- **The F2P Economy State Machine (`economy_lint`):**
  - Mathematical sink-to-faucet verification ($F < S$) ensuring resource scarcity by Day 4.
  - Currency decoupling (EUR $\to$ Gems $\to$ Energy).
  - Hard and soft pity binomial drop guarantees.
  - The \$0.99 "starter anchor" friction-breaker pack.

### 2. The MetaCraft Sludge Paradox: The Meta-Game
> **The Paradox:** Once GameCraft is public, the cost of generating a polished, monetized game drops to zero. Any developer can clone the repo and flood stores with 2,000 games by next week.

When production becomes free:
1. **The Store Becomes an Ocean:** Platforms (Steam, App Store, itch.io) will be flooded with algorithmic games. Valve's \$100 fee per app and curation algorithms become the immediate choke point.
2. **Value Migrates to Curation & Community:** In an era of infinite games, gameplay is no longer the scarce asset. **Attention, taste curation, and social proof** are the only scarce assets left.
3. **The Real Meta-Game:** You don't win by publishing 2,000 unpromoted games. You win by controlling the **curation oracle**:
   - Building the verified "GameCraft Seal of Quality" (games passing strict feel and crash gates).
   - Livestream-ready dynamic hooks (games that produce viral 10-second TikTok/Twitter replay clips via `trace_record`).
   - Community-driven seeds and speedrun leaderboards.

---

## Gate Summary for Phases 6–8

- **Phase 6:** All 12 games pass `feel_lint` with measurable coyote time and squash-stretch springs active in trace recordings.
- **Phase 7:** Native Bevy build boots on Windows/Linux; Godot export passes `steam_deck_lint` at 1280×800.
- **Phase 8:** Steam packaging targets output byte-complete asset bundles (store cards, DLC packs, and cloud save schemas).
