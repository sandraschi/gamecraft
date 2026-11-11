# GameCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(gamecraft-core/bevy)  (gamecraft-mcp)       (gamecraft-desktop)   (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Simulation & Replay Engine (`gamecraft-core`, `gamecraft-replay`, `gamecraft-bevy`)
- [x] L1 `Command::dispatch` universal game execution bus.
- [x] `.gcraft` replay document schema and tamper-evident state hashing.
- [x] Bevy 3D integration with bit-identical schedule replay parity.
- [x] glTF scene graph inspector and spatial bounding box queries.
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`gamecraft-mcp`)
- [x] Unified CLI `gamecraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0 (12 tools).
- [x] Replay recording, verification, and automated scene linting.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`gamecraft-desktop`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` workstation with Bevy viewport.
- [x] Command palette (`Ctrl+Shift+P`) with prefilled parameters.
- [x] Interactive scene tree, replay scrubber, and spatial query visualizer.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About GameCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release binary packaging and NSIS installer.
