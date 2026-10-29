# GameCraft High-Impact Horizons

**Status:** HORIZON SPECIFICATION  
**Date:** 2026-10-29  
**Role:** Strategy & Architecture (Antigravity / Sandra)  
**Fence:** Fenced behind active MCP Toolset implementation (`tools-set-plan.md` Phases 0–4).

---

## 1. Executive Context

GameCraft Phase 5 achieved mechanical proof of generality: 12 distinct Godot 4.4 game archetypes generated with 0 lint errors, 600-frame trace replay wins, and clean headless boots. 

However, all 12 archetypes share a single operational constraint: **every entity is statically pre-placed in a single `main.tscn` scene at compile time**.

To graduate from bounded single-screen arcade toys to scalable, living virtual worlds, GameCraft requires five architectural expansions. This specification defines those horizons and maps them cleanly into future project phases without disturbing the active MCP toolset build.

---

## 2. The Five High-Impact Horizons

```
[Phase 5: Single-Scene Depot (12/12 Done)]
               |
               v
[Active: MCP Toolset Expansion (Phases 0-4)]
               |
               +-----------------------+-----------------------+
               |                       |                       |
               v                       v                       v
     Horizon 1: Dynamic Pooling   Horizon 3: Multi-Scene   Horizon 4: In-Process Audio
     (Endless Runner / #13)       (Project Graph / States) (sfx_synth Rust Engine)
               |                       |                       |
               +-----------------------+-----------------------+
               |
               v
     Horizon 2: Open World Partitioning
     (Chunk Streaming, Seam Linting, 3D Overworld)
               |
               v
     Horizon 5: Real-Time Script Linter
     (GDScript AST / Syntax Validation)
```

---

### Horizon 1: Endless Runner & Dynamic Object Pooling (Archetype #13)

* **Target Phase:** Phase 5.5 / Early Phase 6 (Depot Archetype #13)
* **The Problem:** The current `depot/games/runner` is finite: reach $x = 38$ in 415 frames. It does not prove that GameCraft can manage dynamic entity lifecycles.
* **Architecture:**
  1. **Runtime Chunk Spawner (`chunk_pool.gd`):** Maintains an active window of 3 ground/obstacle chunks.
  2. **Object Pooling Lifecycle:** `VisibleOnScreenNotifier2D` $\to$ on `screen_exited`, recycles the trailing chunk to $x_{\text{front}} + \text{stride}$ instead of allocating new nodes.
* **Verification Invariant:**
  - Standard trace win conditions ($x \ge \text{target}$) do not apply.
  - The verification gate becomes: **Survive $N = 20$ seeded procedural chunks without collision and with zero memory/node-count growth**.

---

### Horizon 2: Open World Partitioning & Seamless Streaming

* **Target Phase:** Phase 7 (Universal Backend & Decoupled Scene IR) & Phase 11 (Mass Simulation)
* **The Problem:** True exploration games cannot be serialized into one `.tscn` file without choking the editor and blowing context limits.
* **Architecture:**
  ```
  world_root.tscn
    ├── Player (CharacterBody3D)
    ├── WorldStreamer.gd (Distance-based chunk loading)
    └── ActiveGrid/
          ├── chunk_0_0.tscn (Terrain + Static Colliders)
          ├── chunk_0_1.tscn
          ├── chunk_1_0.tscn
          └── ...
  ```
  1. **Sub-Scene Streaming:** Chunks are independent `.tscn` files loaded dynamically via `ResourceLoader.load_threaded_request`.
  2. **Boundary Seam Linter (`seam_lint`):** Validates that perimeter vertices and collision AABBs of chunk $(x, y)$ match chunk $(x+1, y)$ within $10^{-4}$ tolerance, preventing players from falling through world tile seams.
  3. **Streaming Patrol Trace:** Simulates an agent walking across chunk boundaries to prove that loading/unloading chunks causes zero physics hitching or collision dropouts.

---

### Horizon 3: Multi-Scene Flow & Project State Graph (`project_graph`)

* **Target Phase:** Phase 7 (The Universal Backend & Platform Harness)
* **The Problem:** Real games are state machines across multiple scenes, not isolated levels.
* **Architecture:**
  1. **Scene Transition Graph:**
     $$\text{Splash} \longrightarrow \text{MainMenu} \longleftrightarrow \text{Settings}$$
     $$\text{MainMenu} \longrightarrow \text{LevelSelect} \longrightarrow \text{Level\_01} \longrightarrow \text{Victory/GameOver}$$
  2. **Validation Rules:**
     - Static validation of all `change_scene_to_file("res://...")` calls against actual scene UIDs.
     - Detects unreachable scenes (orphans) or dead-end states lacking restart/exit transitions.

---

### Horizon 4: Pure-Rust Chiptune Synthesizer (`sfx_synth`)

* **Target Phase:** Phase 6 (Pass D: Audio Aesthetics)
* **The Problem:** Depot games are mechanically verified but completely silent. Sourcing or bundling external `.wav` files breaks GameCraft's self-contained determinism.
* **Architecture:**
  1. **Procedural Waveform Engine:** Pure-Rust synthesis of square, sine, triangle, and white noise waveforms with ADSR envelopes and frequency sweeps (similar to `sfxr`).
  2. **Standard Preset Generators:**
     - `jump()`: Fast upward frequency sweep (square).
     - `coin()`: Two-tone pentatonic chime (sine).
     - `hit()`: Rapid noise burst + pitch drop.
     - `explosion()`: Low-pass filtered noise with long decay.
  3. **Zero-Asset Emission:** Emits 16-bit PCM `.wav` directly into `res://audio/` and binds `AudioStreamPlayer` nodes automatically during archetype scaffolding.

---

### Horizon 5: Real-Time GDScript Syntax Linter (`script_lint`)

* **Target Phase:** Future MCP Toolset Expansion (Phase 5+)
* **The Problem:** `scene_lint` only validates `.tscn` structural tags, collision shapes, and resource IDs. Script syntax errors or missing function targets are only caught when headless Godot runs, adding a multi-second roundtrip.
* **Architecture:**
  1. Integrate a lightweight tree-sitter or GDScript token/AST parser inside `gamecraft-godot`.
  2. Flags undefined identifier references, syntax errors, and missing signal receiver signatures in sub-milliseconds.

---

## 3. Phase Integration Mapping

| Horizon | Primary Focus | Target Document & Phase |
|---|---|---|
| **#1 Endless Runner** | Object pooling, dynamic lifecycles | `depot/` Archetype #13 (Phase 5.5 / 6) |
| **#2 Open World** | Multi-scene streaming, boundary seam lint | `PLAN-phase6-phase8.md` (Phase 7) & `PLAN-phase9-phase11.md` (Phase 11) |
| **#3 Multi-Scene Graph** | Scene flow state machines | `PLAN-phase6-phase8.md` (Phase 7 Platform Harness) |
| **#4 Procedural Audio** | In-process waveform generation (`sfxr`) | `PLAN-phase6-phase8.md` (Phase 6 Pass D) & `getsometaste.md` |
| **#5 Script Linter** | In-process sub-millisecond GDScript AST lint | `tools-set-plan.md` (Future MCP Tools) |
