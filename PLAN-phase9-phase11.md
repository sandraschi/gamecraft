# PLAN Phases 9–11: The AAA Horizon — Asset Foundry, Soul Engine, and Mass Simulation

**Status:** HORIZON SPEC (2026-10-29).  
**Fence:** **Do NOT start until Phases 5 through 8 are `[STATUS: SUCCESS]`.**  
**Focus:** Pure game-building engineering. How GameCraft scales from 12 arcade archetypes to AAA-scale virtual worlds.

---

## The AAA Engineering Paradox

Traditional AAA production is an organizational brute-force equation:
$$\text{AAA} = \$200\,\text{Million} + 500\,\text{Engineers \& Artists} + 6\,\text{Years of Slog}$$

Because GameCraft already solved deterministic spatial compilation (`gamecraft-core`) and verifiable physics/mechanics (`gamecraft-forge`), the remaining barrier to AAA fidelity is not code logic—it is **volumetric density**: high-fidelity 3D assets, dynamic character behavior/voice, and massive world simulations.

Phases 9 through 11 solve these three volumetric bottlenecks as deterministic pipeline stages.

---

## Phase 9: The Neural Asset Foundry (`gamecraft-assets`)

**Goal:** Eliminate the 200-person art department. Synthesize, optimize, and bind high-poly 3D assets, PBR materials, and rigged animations directly into the Scene IR.

### 1. Volumetric Synthesis & Topology Compiler
- **Diffusion & Splat to Mesh:** Accept textual briefs or reference imagery $\to$ generate 3D Gaussian Splats or neural radiance fields $\to$ extract watertight marching cubes.
- **Automated Quad Retopology & LOD Cascades:**
  - Automated edge-loop retopology converting raw millions of triangles into clean quad topologies.
  - Cascading Level of Detail generation: LOD0 (hero mesh, 35k polys) down to LOD4 (impostor/billboard, 200 polys) with automatic screen-space error bounds.
- **PBR Texture Baking:** Synthesize synchronized $4\text{K}$ texture sets: Albedo, Normal (tangent-space), Roughness, Metallic, and Ambient Occlusion.

### 2. Skeletal Auto-Rigging & Motion Matching
- **Automated Bone Weighting:** Raycast interior medial-axis skeletons into humanoid and quadruped meshes; compute heat-diffusion vertex skinning weights.
- **Motion-Matching Database:** Instead of hardcoded state machines, maintain a database of locomotion vectors (walk, sprint, vault, skid, strafe). The runtime queries the database every frame to blend the mathematically optimal animation pose.

---

## Phase 10: The Living Soul Engine (`gamecraft-soul`)

**Goal:** Graduate from pre-scripted state trees to dynamic, emergent NPC behavior, voice prosody, and persistent episodic memory without breaking determinism.

### 1. Episodic NPC Memory & Local Embedding Spaces
- **Vectorized Spatial Memory:** Each NPC maintains an in-memory HNSW vector index of recent observations (e.g., *"Player stole bread at frame 4200"*, *"Player carries iron sword"*).
- **Behavior Utility Grids:** Replaces brittle decision trees with utility theory. NPCs evaluate environmental affordances (Hunger, Danger, Loyalty, Greed) weighted against their personality vector.

### 2. Dynamic Narrative & Reactive Voice Synthesis
- **Branching Quest Graphs:** Quests modeled as directed acyclic graphs with dynamic precondition solving (if NPC $A$ dies, the quest solver reroutes objectives to NPC $B$ or an environmental note).
- **Procedural Prosody & Dialogue:** Integration with local neural speech synthesis (0-latency ONNX acoustic models). Emits spoken dialogue with variable pitch, emotional inflection, and dynamic lip-sync phonemes matched to facial blend shapes.

---

## Phase 11: Mass Simulation & Living World ECS (`gamecraft-sim`)

**Goal:** Scale the scene graph from hundreds of nodes to 10,000+ active simulated entities, fluid dynamics, and destructible environments.

### 1. High-Density Data-Oriented Architecture (ECS)
- **Archetype ECS Kernel:** Decouple massive entity populations from traditional hierarchical scene trees. Entities (crowds, projectiles, foliage, debris) reside in contiguous cache-friendly arrays.
- **Multi-Threaded Spatial Partitioning:** SIMD-accelerated bounding-volume hierarchies (BVH) updating 10,000 transforms and AABBs in $< 2\,\text{ms}$ per frame.

### 2. Destructible Environments & Ecosystem Cycles
- **Voxel/Convex Decomposition:** Dynamic destruction—walls and pillars fracture along Voronoi shatter planes when subjected to impulse forces exceeding shear thresholds.
- **Closed-Loop Micro-Economies & Food Webs:** Wildlife, merchants, and factions run headless state loops balancing predator/prey and commodity supply/demand.

---

## The Unified Horizon Pipeline

```
[Prompt Brief / World Seed]
            │
            ▼
[gamecraft-core: Spatial Compiler IR]
  - Validates transform hierarchies, collision layers, and raycast reachability
            │
    ┌───────┼──────────────────────────┐
    ▼       ▼                          ▼
[Phase 9] [Phase 10]               [Phase 11]
Neural    Living Soul              Mass ECS
Asset     (Episodic Memory,        (10k Entities,
Foundry   Voice Prosody,           Destruction,
(LOD0-4,  Dynamic Quest            Voxel Fracture)
Auto-Rig) DAGs)
    │       │                          │
    └───────┼──────────────────────────┘
            │
            ▼
[gamecraft-juice: Aesthetics Pass (Phase 6)]
            │
            ▼
[Universal Exporter: Godot 4 / Bevy / Native Steam Bundle]
  - Emits a fully playable, tactile, living AAA virtual world in an afternoon.
```
