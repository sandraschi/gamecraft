# GameCraft — Product Requirements Document (PRD)

**Document Identifier:** `CRAFT-PRD-2026-GAME-001`  
**Version:** 1.0.0  
**Status:** Active  
**Domain:** Autonomous Game Engine Studio, Bevy 3D Simulation & Replay Verification  
**Standard Port:** 14122  
**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  

---

## 1. Executive Summary & Problem Statement

Modern game development tools (Unity, Unreal) suffer from proprietary cloud lock-in, multi-gigabyte editor overheads, and fragile non-deterministic physics that break automated agentic testing and regression verification.

**The GameCraft Solution:**
GameCraft is a pure-Rust, deterministic 3D game engine atelier and simulation testbed:
- Deterministic `.gcraft` replay document (seed, config, input log, state hash).
- Bevy 3D integration with bit-identical replay parity.
- Scene linting, glTF / Godot scene verification, and spatial bounding box queries.
- Universal command registry (`commands --json`, `describe <id>`) and MCP stdio server.

---

## 2. Core Functional Requirements

1. **Simulation & Replay Engine (`gamecraft-replay`, `gamecraft-sim`, `gamecraft-bevy`):**
   - Deterministic simulation loops with state hashing and tamper verification.
   - Replay recording and playback verification.
2. **Scene & Asset Geometry (`gamecraft-gltf`, `gamecraft-godot`, `gamecraft-forge`):**
   - glTF scene graph inspection, mesh bounding box calculation, spatial queries.
   - Godot bridge integration with headless validation.
3. **Universal Command Bus (`Command::dispatch`):**
   - 100% of CLI, MCP, and UI actions dispatch through the universal command registry.

---

## 3. UI/UX Architecture & Menu Bar Standard

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Game Workstation):** Persistent 3D Bevy game simulation viewport with scene hierarchy tree, replay timeline, and spatial inspector. Top quick button row acts as accelerators into the `View` menu. To add a new scene tool or simulation entity:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Scene / Replay toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `File`: New Game Scene (`Ctrl+N`), Open Scene / Replay (`Ctrl+O`), Open Recent ▸, Save Scene (`Ctrl+S`), Export glTF / Replay (`Ctrl+E`), Close (`Ctrl+W`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut, Copy, Paste, Select All, Preferences (`Ctrl+,`).
  - `View`: 3D Game Viewport, Scene Tree Hierarchy, Replay Timeline Scrubber, Spatial Inspector, Performance HUD, Theme (`Dark`/`Light`).
  - `Insert`: Add Entity / Mesh, Add Light / Camera, Insert Collision Shape, Record Replay Marker.
  - `Modify`: Transform Entity (Translate/Rotate/Scale), Edit Physics Properties, Adjust Simulation Tick Rate.
  - `Commands`: Run Scene Lint (`Ctrl+L`), Run Spatial Query, Verify Replay Parity, Step Simulation, Build Game.
  - `Window`: Reset Layout, 3D Game Viewport, Scene Inspector Drawer, Command Palette (`Ctrl+Shift+P`).
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+Shift+P` / `Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About GameCraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.
