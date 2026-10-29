# GameCraft / Storytold ArtCraft Automation Parity Specification

**Status:** SPECIFICATION / AUDIT  
**Date:** 2026-10-29  
**Role:** Architecture & Standards Alignment (Antigravity / Sandra)  
**Standard Reference:** `mcp-central-docs/standards/rust-artcraft/CRAFT_COMMAND_BUS.md`  
**Fence:** Fenced behind active MCP Toolset implementation (`tools-set-plan.md` Phases 0–4).

---

## 1. Executive Summary

The Storytold ArtCraft architecture (proven across Brandon Thomas's 13 Crafting Apps: *PhotoCraft*, *VectorCraft*, *CADCraft*, *FigCraft*, etc.) defines the fleet standard for AI-driven desktop applications. 

Its defining breakthrough is the **Two-Plane Command Bus**: an application that functions identically as a headless batch compiler (Plane 1) and as an interactive desktop GUI whose visible canvas can be driven in real time by an AI agent over an attach port (Plane 2).

This document audits GameCraft against the Storytold ArtCraft standard, details what is currently emulated, identifies the remaining architectural gap (Plane 2 Live Window Attachment), and provides the blueprint to achieve 100% parity.

---

## 2. Four Shapes of App Automation

| Shape | Mechanism | Example | Strengths & Limits |
|---|---|---|---|
| **Headless Batch** | Client runs CLI, writes file, exits. | `gamecraft-cli export` | Fast, deterministic; no live human visual feedback. |
| **Vendor Scripting Bridge** | Calls app-specific scripting socket while open. | Blender `bpy`, Photoshop ExtendScript | Brittle, vendor API drift, external dependencies. |
| **Computer-Use / CUA** | AI drives raw pixels, mouse clicks, keystrokes. | Anthropic CUA, Playwright | Works anywhere, but fragile, non-deterministic, zero semantic model. |
| **Command Bus + Attach (ArtCraft)** | App exposes documented command registry over a control port; MCP server proxies commands to it. | `figcraft-cli mcp --attach 7979` | **Semantic, deterministic, fully visible.** Human watches AI construct scene live. |

GameCraft currently implements **Shape 1 (Headless Batch)** and is expanding into full headless tooling via `tools-set-plan.md`. Full ArtCraft parity requires adding **Shape 4 (Command Bus + Attach)**.

---

## 3. Parity Matrix: GameCraft vs. Storytold ArtCraft

| ArtCraft Requirement | Storytold Standard (`mcp-central-docs`) | GameCraft Current Status | Gap |
|---|---|---|---|
| **Language & Containment** | Pure-Rust, zero `unsafe`, deny `unwrap`/`panic`, WASM probe. | **100% Parity.** 6-gate bar enforced across workspace. | None. |
| **Unified Command Bus** | Typed `Command` enum with `execute`, `undo`, `serialize_json`. | **100% Parity.** Implemented in `crates/gamecraft-commands`. | None. |
| **No UI-Only Actions** | Every mouse/palette GUI action has an identical programmatic command. | **100% Parity.** Audited in `tools-set-plan.md` §2. | None. |
| **Plane 1 (Headless)** | CLI/MCP compiles scenes, simulates physics, and exports artifacts headlessly. | **100% Parity.** `scene_lint`, `spatial_query`, `export_scene`, and planned `scene_verify`. | None. |
| **Plane 2 (Live Attach)** | Desktop GUI exposes `--control <PORT>`; MCP server proxies live edits via `--attach <PORT>`. | **Not Implemented.** `gamecraft-desktop` has no control socket; mutations occur on disk only. | **Live control bridge missing.** |

---

## 4. The Plane 2 Architecture (Live Window Attachment)

In full ArtCraft mode, the human opens the GameCraft desktop application to inspect a scene. An AI agent in Claude Desktop, Cursor, or Antigravity connects to that live session:

```
+---------------------------------------------+
| Client (Claude / Antigravity / Cursor)      |
+----------------------+----------------------+
                       | stdio (JSON-RPC)
                       v
+---------------------------------------------+
| crates/gamecraft-mcp (running --attach 7979)|
+----------------------+----------------------+
                       | localhost TCP (JSON-RPC frames)
                       v
+---------------------------------------------+
| apps/gamecraft-desktop (running --control)   |
|                                             |
|  [Background TCP Listener Thread]           |
|            |                                |
|            v (mpsc::channel)                |
|  [GUI Main Loop / CommandStack]             |
|            |                                |
|            v                                |
|  [egui 3D Viewport] -> Repaints frame live  |
+---------------------------------------------+
                       |
                       +---> Human watches nodes appear, transforms
                             move, and colliders adjust in real time!
```

---

## 5. Technical Specification for Plane 2 Parity

### Step A: Desktop Control Listener (`apps/gamecraft-desktop`)

1. **CLI Flag:** Add `--control [PORT]` to `apps/gamecraft-desktop` (default port: `7979`).
2. **Listener Thread:** When `--control` is supplied, spawn a background `std::net::TcpListener` bound strictly to `127.0.0.1:<PORT>`.
3. **Dispatch Channel:**
   ```rust
   // apps/gamecraft-desktop/src/control.rs
   pub enum ControlMessage {
       ExecuteCommand(Box<dyn Command>),
       Undo,
       Redo,
       GetSceneGraph(mpsc::Sender<SceneGraphDto>),
   }
   ```
4. **Main Loop Drain:** On every `egui` frame update, the desktop drains pending `ControlMessage`s, executes them on the active `CommandStack`, and calls `ctx.request_repaint()`.

### Step B: MCP Proxy / Attach Mode (`crates/gamecraft-mcp`)

1. **CLI Flag:** Add `--attach <PORT>` to `gamecraft-mcp`.
2. **Dual-Mode Dispatch in `scene_edit`:**
   - **Standalone Mode (default):** Reads `.tscn` from disk, applies commands, writes `.tscn` back with `.bak` and atomic rename.
   - **Attach Mode (`--attach <PORT>`):** Connects to `127.0.0.1:<PORT>` over TCP. Forwards `scene_edit` commands directly to the running desktop window. The desktop executes them in memory and saves when requested.

### Step C: Security & Safety Contracts

1. **Localhost Only:** Socket binds exclusively to `127.0.0.1`. Remote IP connections are immediately dropped.
2. **Opt-In Only:** Desktop only opens the socket if `--control` is explicitly passed by the human operator.
3. **Error Feedback:** If `--attach` fails to reach the control port, the MCP tool returns a clean error:
   `"gamecraft-desktop is not running with --control 7979. Start the desktop app or omit --attach to edit on disk."`

---

## 6. Phase Fencing & Roadmap

1. **Current Focus (Active):** Finish `tools-set-plan.md` Phases 0–4 (Extract `gamecraft-sim`, implement `scene_inspect`, `scene_verify`, `scene_edit`, `forge_archetype`).
2. **Plane 2 Implementation:** Scheduled as an enhancement directly following Phase 3 (`scene_edit`), requiring:
   - PR 1: `--control` TCP listener in `apps/gamecraft-desktop`.
   - PR 2: `--attach` flag and TCP forwarding in `crates/gamecraft-mcp`.
