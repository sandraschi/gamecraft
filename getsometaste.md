# GetSomeTaste: The GameCraft Taste, Feel & Aesthetics Cookbook

> *"Spatial compiler now, world compiler as horizon. When someone can `scene_lint` for boredom and `trace_record` for meaning, then we can talk about godhood. Until then, exocortex builds arenas."*  
> — Sandra Schipal, *On MetaCraft* (October 2026)

This document is the formal recipe for graduating GameCraft from a pure spatial verifier (0 errors, grounded colliders, connected signals) into an engine capable of compiling **tactile sensation, juice, visual harmony, and taste** into games in an afternoon.

---

## 1. Canonical Foundations (The Four Pillars)

1. **Steve Swink — *Game Feel: A Game Designer’s Guide to Virtual Sensation* (2008)**  
   Formalizes tactile sensation into three computable tiers:
   - **Real-time control:** Sub-100ms feedback, high input responsiveness, velocity damping.
   - **Spatial simulation:** Viscosity, inertia, gravity curves, and tactile friction.
   - **Sensory reinforcement:** Sound, visual distortion, and haptics confirming physical contact.
2. **Martin Jonasson & Petri Purho — *Juice It or Lose It* (Nordic Game 2012)**  
   Proved that a sterile, mechanically dry game (e.g. basic Breakout) transforms into an addictive experience purely through non-functional perceptual amplification (tweens, particle bursts, screen shake, hit-stop).
3. **Maddy Thorson — *Celeste Game Feel Heuristics***  
   Codified the subtle mathematical lies game engines must tell to feel "fair": coyote time, jump buffering, corner correction, and apex gravity dampening.
4. **Robin Hunicke, Marc LeBlanc, Robert Zubek — *MDA Framework* (2004)**  
   Formal bridge from **Mechanics** (rules, variables) $\to$ **Dynamics** (runtime systems) $\to$ **Aesthetics** (targeted emotional responses: Sensation, Fantasy, Narrative, Challenge, Discovery, Expression).

---

## 2. The Four Aesthetic Compilation Passes

To inject taste into generated games deterministically, GameCraft can apply four automated passes on top of the validated scene graph:

### Pass A: Tactile Mechanics (Forgiving Physics)
Raw rigid-body physics feel stiff and punishing. A `feel_lint` pass enforces forgiving mechanics in character controllers:
- **Coyote Time:** Allow jumping 4–6 frames (60–100ms) *after* the character’s AABB walks off an edge.
- **Jump Buffering:** Cache jump keypresses 5 frames *before* touchdown; fire the jump on the exact first grounded frame.
- **Apex Float:** Halve gravity when vertical velocity is near zero ($|v_y| < 1.5\,\text{m/s}$) for 3 frames, providing air-control precision.
- **Corner Correction:** If an upward-moving AABB clips an overhead ledge by $\le 4$ pixels, raycast nudge the body sideways into open space instead of abruptly canceling vertical momentum.
- **Variable Jump Cut:** Releasing the jump button early immediately multiplies upward velocity by $0.5$, giving fine analog height control from digital keys.

### Pass B: The Juice Middleware (Visceral Feedback)
Every collision, destruction, or state transition emits secondary sensory effects via reusable Godot nodes:
- **Trauma-Based Screen Shake (Squirrel Eiserloh Model):**  
  Camera offset derived from non-linear trauma decay:
  $$\text{Offset} = \text{Trauma}^2 \times \text{PerlinNoise}(t)$$
  $$\text{Trauma} \leftarrow \max(0, \text{Trauma} - \text{DecayRate} \times \Delta t)$$
  Small impacts add $+0.1$; explosions or deaths add $+0.5$.
- **Hit-Stop (Micro-Freeze):**  
  On impactful hits (combat strike, brick shatter), pause engine time scale (`Engine.time_scale = 0.0`) for 2–4 frames (30–60ms), then resume. Gives weight to collisions.
- **Squash and Stretch:**  
  Elastic scale deformation during acceleration and landing:
  - Landing: Scale $(X \times 1.25, Y \times 0.75)$, restoring to $1.0$ over 100ms via an ease-out spring.
  - Jumping: Scale $(X \times 0.8, Y \times 1.2)$.
- **Deterministic Particle Bursts:**  
  Attach lightweight GPU particle emitters: footstep dust poofs, spark bursts on metal/bullet contact, and confetti on goal triggers.

### Pass C: Color Theory & Visual Harmony (*Old Man's Journey* / *Gris*)
Indie gems achieve artistic distinction through **deliberate constraint**, not rendering complexity:
- **Harmonic Palette Palettes (Oklab Color Space):**  
  Ban raw primary RGB. Select from curated harmonic sets:
  - *Nostalgic / Melancholy (Old Man's Journey):* Warm analogous earth tones, terracotta, muted sage, washed denim.
  - *Dreamlike / Architectural (Monument Valley):* Pastel triadic schemes with high luminance and low saturation.
  - *Minimalist Cyber (Hyper Light):* High-contrast split-complementary neon accents against deep slate backgrounds.
- **Atmospheric Depth & Parallax:**  
  Automatically inject 3 background parallax layers with distance scaling ($0.2\times, 0.5\times, 0.8\times$) and subtle atmospheric fog tinting toward the horizon.
- **Cinematic Framing:**  
  Subtle vignette post-processing shader, clean letterbox framing (2.39:1 aspect ratio), and generous negative space around gameplay arenas.

### Pass D: Dynamic Audio Aesthetics
- **Pentatonic Pitch Quantization:**  
  Quantize procedurally triggered sound frequencies to a minor or major pentatonic scale—ensuring any sequence of collision tones or chimes remains harmonically pleasing.
- **Micro-Pitch Jitter:**  
  Modulate every repeating sound effect (footsteps, ball bounces, laser fire) by $\pm 5\%$ random pitch jitter to banish the machine-gun effect.

---

## 3. Pipeline Integration: Phase 6 Horizon

```
[Raw Scene Specification]
           │
           ▼
[gamecraft-core: Spatial Compiler]
  - 0 clipping, grounded AABBs, valid UIDs, verified signal wiring
           │
           ▼
[gamecraft-juice: Aesthetics & Taste Pass]
  - Injects Pass A (coyote time, buffering) into character GDScripts
  - Injects Pass B (trauma camera, hit-stop, squash)
  - Injects Pass C (Oklab palette material decks, parallax, vignette)
           │
           ▼
[Deterministic Godot 4.3 Export]
  - Playable, verified, and dripping with indie studio feel in an afternoon.
```
