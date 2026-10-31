# GameCraft — Comprehensive Assessment & Improvement Suggestions

- **Date:** 2026-10-31T03:35:00+01:00
- **Author tag:** Antigravity (Gemini 3.8 Flash)
- **Target Repo:** `D:\Dev\repos\gamecraft` (evaluated at commit `88b718e` / `c9e8b7b`, v0.12.0)
- **Role:** Independent read-only assessment, peer review, and prioritized improvement roadmap (Kibitzer).
- **Companion / Preceding Docs:** 
  - `ASSESSMENT-antigravity-2026-10-29.md` (initial architecture audit, pre-forge)
  - `ASSESSMENT-claude-2026-10-29.md` & `ASSESSMENT-claude-2026-10-29-r2.md` (rounds 1 & 2)
  - `KIBITZ.md` (adjudicated dialogue and queue)
  - `docs/LLM_AS_GAME_DESIGNER.md` (working notes on the web family)

---

## 1. Executive Verdict

GameCraft has evolved in under 48 hours from a pure spatial linter prototype into a dual-engine autonomous game synthesis platform:
1. **The Godot Depot Track:** A pure-Rust deterministic compiler (`crates/gamecraft-forge`, `gamecraft-godot`, `gamecraft-sim`) that compiles high-level JSON specifications into playable Godot 4.4 projects without letting language models write unverified GDScript.
2. **The Web Family Track (`web/`):** A pragmatic companion track (Three.js + Bun/Node 24) where models write full browser games verified by headless Chromium self-play, state checkpoints, and quantitative heuristic bots (`selfTest()`).

The codebase is disciplined:
- Rust test suite is 100% green (**88 passed**, 0 failed across all 8 workspace members).
- `clippy::pedantic` passes with `-D warnings`.
- `wasm32-unknown-unknown` portability probe on `gamecraft-core` remains clean.
- In-engine Godot verification (`just verify-depot`) has advanced from **6/15 to 11/15 in-engine wins** (73% verified winnable).
- Web track (`just web-verify`) passes **2/2 games** (`kaffeehaus` at 45,001 steps, `mossbrook` at 11,537 steps across 14 checkpoints).

Four key friction points and opportunities require builder attention:
1. **The 4 Unproven Depot Games:** `breakout`, `flappy`, `topdown-shooter`, and `tower-place` still report `NO WIN MARKER` in headless Godot runs.
2. **Environment Path Ergonomics for `web`:** `just web-verify` and `web/scripts/verify.ts` fail on Windows environments when `bun` is installed at `~/.bun/bin/bun.exe` but not exported to the ambient `$env:Path`.
3. **MCP Toolset Coverage Gap:** While CLI and desktop have advanced rapidly, MCP clients (Claude Desktop, Antigravity, Cursor) still only have 4 read/export tools. The planned `scene_verify` and prompt-to-game compiler are absent from the MCP schema.
4. **Plane 2 Live Window Attachment:** The live control bridge (`--control` / `--attach`) remains unbuilt, leaving agent interaction tethered to cold file edits rather than real-time egui viewport control.

---

## 2. Empirical Verification Matrix

Every gate was executed independently during this evaluation:

| Gate / Command | Measured Result | Evaluation & Ground Truth |
|---|---|---|
| `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic` | **CLEAN** | Zero warnings. Strict denial of `unwrap_used`, `expect_used`, `panic`, and `as` casts holds. |
| `cargo fmt --all --check` | **CLEAN** | Formatting adheres strictly to rustfmt rules. |
| `cargo check -p gamecraft-core --target wasm32-unknown-unknown` | **GREEN** | Core domain remains zero-dependency with no engine/OS leakage. |
| `cargo test --workspace` | **88 passed, 0 failed** | Breakdown: `commands` 6, `core` 14, `desktop` 7, `forge` 19, `godot` 10+2, `mcp` 22, `sim` 8. |
| `pwsh scripts/verify-depot.ps1` (Godot 4.4 headless, 3600 frame budget) | **11 WIN / 4 NO WIN** | **Wins:** `chessboard-move`, `driving`, `flight`, `maze`, `night-forest`, `platformer`, `pong`, `runner`, `snowy-mountain-pass`, `sokoban`, `talk-inventory`.<br>**No Win Marker:** `breakout`, `flappy`, `topdown-shooter`, `tower-place`. |
| `node web/scripts/verify.ts` (Playwright SwiftShader headless Chromium) | **2/2 PASS** | `kaffeehaus`: PASS (steps=45,001, errors=0, selfTest passed).<br>`mossbrook`: PASS (steps=11,537, errors=0, 14 checkpoints saved to `shots/`, stealth self-test passed). |
| Ambient `just web-verify` | **FAILED (Missing PATH)** | Fails with `'bun' is not recognized` unless `~/.bun/bin` is prepended to `$env:Path` in `justfile`. |

---

## 3. Detailed Architectural Assessment

### 3.1 Prompt-to-Game Compilation (`gamecraft-forge::create`)
- **Coverage:** Prompt compilation (`gamecraft-cli new --prompt "..."`) now supports **8 of 13 archetypes**: `platformer`, `runner`, `pong`, `maze`, `flight`, `driving`, `chess`, and `talk`.
- **Architectural Strength:** The "Spatial Compiler" design principle is completely vindicated. By enforcing that LLMs output structured `GameSpec` data rather than raw GDScript, hallucinations are caught at spec validation time. A malformed prompt causes a clean rejection, never a silently broken `.tscn` file.
- **Parametric Elegance:** Recent commits (`83028d0`, `88b718e`, `f577c16`) transformed static const courses into parametric generators:
  - `chess`: `knight_walk` generates legal non-revisiting tours; target pads derive from landings.
  - `talk`: `talk_route(visits)` calculates deterministic waypoints and thresholded win conditions.
  - `flight`: Course lengths scale autopilot frame budgets automatically.

### 3.2 The Web Family (`web/`)
- **Complementary Thesis:** Rather than forcing the rigid Rust compiler onto narrative vibe games, the web family embraces model-written Three.js while enforcing rigid behavioral contracts via `window.__gamecraft`.
- **The Heuristic Gate Breakthrough:** `kaffeehaus`'s `selfTest()` (`goodBot` > `randomBot` > `idleBot`) is a groundbreaking method for verifying gameplay mechanics without human playtesting. Over a 5-day campaign, good play achieves ~€930 while random clickers cap at ~€720. That measurable delta proves meaningful agency.
- **Negative Self-Tests:** In `mossbrook`, the stealth test demonstrates that walking without the Hush flame alerts the Keeper in 2.7s, while Hush allows safe transit. This eliminates false-positive passes from trivial colliders.

### 3.3 The egui/wgpu Desktop Studio (`gamecraft-desktop`)
- Native 3D viewport displays true shaded triangles with directional lighting and albedo colors.
- The 15-game depot selector with one-click "Start Game" directly fulfills the rapid demonstration requirement.
- Trace scrubbing across the 60-frame ring buffer enables intuitive debugging of physics anomalies.

---

## 4. Key Deficiencies & Vulnerabilities

### Defect 1: Ambient PATH Brittleness in Web Automation
- **Location:** `justfile` lines 86-92 and `web/scripts/verify.ts` line 37.
- **Symptom:** Running `just web-verify` or `node scripts/verify.ts` in fresh PowerShell sessions, NSSM services, or IDE terminals fails with:
  ```
  'bun' is not recognized as an internal or external command
  ```
- **Root Cause:** While `justfile` carefully prepends `$env:USERPROFILE\.cargo\bin` for every Rust recipe, it leaves `web-serve` and `web-verify` bare (`bun run serve`, `node scripts/verify.ts`). On Windows, Bun installs to `%USERPROFILE%\.bun\bin`, which is often absent from non-interactive shell PATHs. Furthermore, `verify.ts` spawns `bun` via `shell: true`, triggering Node deprecation warning `DEP0190`.

### Defect 2: The 4 Unproven Depot Games (Depot Gate Incompleteness)
- **Status:** 4 games fail `scripts/verify-depot.ps1`:
  1. `breakout`: Paddle autopilot policy does not track ball trajectory or brick clearance.
  2. `flappy`: No flapping autopilot policy; bird drops into floor/pipe immediately.
  3. `topdown-shooter`: Autopilot does not seek enemies or fire in their direction; hostiles remain alive.
  4. `tower-place`: Gold is never spent or placement coords are not pulsed; wave never clears.
- **Impact:** While documented honestly in `README.md`, `just verify-depot` remains permanently red in CI/scripts, making automated regression detection noisy.

### Defect 3: MCP Server Stagnation (Tool Asymmetry)
- **Current Tools:** `scene_lint`, `spatial_query`, `scene_inspect`, `export_scene`.
- **Missing Capabilities:**
  - `scene_verify`: Agents must drop into raw PowerShell to check whether a game actually boots and wins in Godot.
  - `forge_new` / `create_game`: Claude Desktop and shell-less clients cannot invoke `gamecraft-cli new` over MCP.
  - `economy_lint` / `simulate_economy`: Phase 8 monetization features exist only as Rust library functions, invisible to MCP tool discovery.

### Defect 4: Plane 2 Live Window Control Unimplemented
- Per `docs/ARTCRAFT_PARITY.md`, the live egui window cannot be attached to via TCP (`--control <PORT>` / `--attach <PORT>`).
- If an agent generates or alters a scene, the human or agent must restart the desktop viewer rather than hot-reloading or querying the active camera in real time.

---

## 5. Prioritized Improvement Suggestions

### Tier 1: Immediate Hygiene & Friction Killers (1-2 Hours)

1. **Fix `justfile` and `web/scripts/verify.ts` Pathing:**
   - In `justfile`, prepend `$env:USERPROFILE\.bun\bin` to `web-serve` and `web-verify`:
     ```just
     web-serve:
         $env:Path = "$env:USERPROFILE\\.bun\\bin;$env:Path"; Set-Location web; bun run serve

     web-verify:
         $env:Path = "$env:USERPROFILE\\.bun\\bin;$env:Path"; Set-Location web; node scripts/verify.ts
     ```
   - In `web/scripts/verify.ts`, resolve `bun` with a fallback to `join(process.env.USERPROFILE || "", ".bun", "bin", "bun.exe")` if bare `bun` is not on `PATH`. Replace `shell: true` with direct executable execution to eliminate `DEP0190`.

2. **Add Exit Summary to `scripts/verify-depot.ps1`:**
   - Print an explicit breakdown ratio (e.g. `[11/15 WIN] [4/15 UNPROVEN]`) so log parsers can track progress numerically.

---

### Tier 2: Core Engineering & In-Engine Wins (1-2 Days)

3. **Autopilot Policies for the Remaining 4 Games:**
   - **`breakout`**: Implement a simple tracking heuristic in `autoplay.gd` (`paddle.x = clamp(ball.x, min, max)`). With ball tracking, 30 bricks clear within ~1200 frames.
   - **`flappy`**: Implement altitude maintenance policy (flap whenever `player.y < next_pipe.gap_center_y`).
   - **`topdown-shooter`**: Implement greedy nearest-enemy targeting and autofire in headless autoplay mode.
   - **`tower-place`**: Place 3 towers at predefined strategic waypoints at frame 0 so incoming creeps are neutralized automatically.
   - *Outcome:* Moves `just verify-depot` to **15/15 WIN**, turning the entire depot gate green!

4. **Expand Prompt Archetypes from 8 to 11:**
   - Wire `sokoban` (grid size, seed, push budget via BFS solver).
   - Wire `breakout` and `topdown-shooter` once their in-engine autopilots are verified winnable.

---

### Tier 3: MCP Server & Client Parity (Phase 2 & 3 Toolset)

5. **Expose `scene_verify` over MCP:**
   - Implement the planned MCP tool that runs Godot headless in a temp sandbox, checks for errors/warnings, monitors stdout for `GameCraft: goal reached`, and returns execution time, frames elapsed, and pass/fail diagnostics.

6. **Expose `create_game` / `forge_spec` over MCP:**
   - Allow shell-less clients to send a JSON `GameSpec` (or a text prompt if a local model is reachable) and receive the compiled project directory path.

7. **Expose `economy_lint` over MCP:**
   - Bring `crates/gamecraft-forge::economy` to the MCP surface, allowing agents to audit virtual economies for faucet dominance, missing starter anchors, and pity progression.

---

### Tier 4: Taste, Juice & Live Studio (Horizons)

8. **Implement `docs/ARTCRAFT_PARITY.md` Plane 2:**
   - Add a lightweight TCP localhost listener in `gamecraft-desktop` and `--attach` in `gamecraft-mcp`.
   - Allows agents to dynamically select nodes, trigger orbit camera poses, and scrub frames while the user watches.

9. **Phase 6 "Taste" Middleware in Headless Compiler:**
   - Begin incorporating Pass A & Pass B from `getsometaste.md` into generated GDScript templates: coyote time (5 frames), jump buffering, and trauma-squared screen shake, protected by regression assertions.

---

## 6. Kibitzer Protocol Pointer

A condensed reference entry pointing to this document has been appended to `KIBITZ.md`. The builder agent may review and adjudicate these items in `KIBITZ_BACKTALK.md` at their next turn.
