# GameCraft MCP Toolset Expansion Plan (rev 3)

**Status:** PROPOSED
**Date:** 2026-10-29
**Target crate:** `crates/gamecraft-mcp`
**Transport:** MCP `2024-11-05`, stdio, newline-delimited JSON-RPC

---

## 1. The test

One question decides every candidate:

> Can a competent agent with file read/write and reasoning produce the same
> answer reliably?

If yes, the tool is **convenience** and must justify itself by removing a real
failure mode (transcription error, context size, non-determinism) - not by saving
keystrokes. If no, it is **necessary**.

Nothing else counts. Not tool count, not client context budget, not what any
other machine has registered.

---

## 2. Decision

| Candidate | Verdict | Why |
|---|---|---|
| **`scene_inspect`** | **Necessary** | Parsing `.tscn` into a parent/child graph with resolved transforms is where agents actually fail: parent path semantics (`"."` vs `"Body"`), unique-name flags, `sub_resource` id binding, packed `Transform3D` decoding. An agent reading the text guesses at these. A tool that returns the parsed graph answers exactly. |
| **`scene_lint`** (exists) | **Necessary** | Encodes rules the agent does not reliably apply: every physics body needs a collision shape descendant, every connection needs a resolvable method, every property reference needs a declared resource, every scene needs a current camera. The resource rule is not hypothetical - it caught a real id drift the same day it was written, after the scene had already been exported. The other three are preventative, which is the cheaper half of the same argument. |
| **`spatial_query`** (exists) | **Necessary** | World-space AABB requires composing the full parent chain of `Mat4`s and unioning subtree geometry. Hand-computing it from text is both tedious and error-prone, and the answer is load-bearing for spawn clearance and overlap checks. |
| **`export_scene`** (exists) | **Necessary** | The value is not the write, it is the guarantees: deterministic `uid://` from path, byte-identical output on repeat, correct Godot literal encoding (typed strings quoted, `ExtResource`/`NodePath` not). An agent hand-writing `.tscn` gets one of those wrong - today's evidence: unquoted strings, over-quoted paths, and `%Name` unique syntax, all three parse errors. |
| **`scene_verify`** (new) | **Necessary** | Three things reasoning cannot establish: (a) whether a scripted input reaches the win state inside the frame budget (mirrored Rust sims), (b) whether Godot accepts the scene (only the engine knows), (c) what the scene actually looks like. (c) is the one this plan was missing: rasterizing a graph is not something an agent can do by reading text, and "does the level look right" is the core question for a spatial compiler. |
| **`scene_edit`** (new) | **Desirable, necessary above ~5 KB** | Below that, `inspect -> edit JSON -> export` is fine. Above it, an agent re-emitting a whole scene invites silent transcription loss - a dropped property looks identical to an intentional deletion. The tool's justification is that it mutates by reference and returns a before/after hash, so nothing is silently rewritten. |
| `signal_graph` | **Cut** | Same parse, same graph, different projection of `scene_inspect`. No capability gained. |
| `scene_connect` | **Cut** | A mutation. Belongs in `scene_edit`'s command list, not as its own tool. |
| `forge_archetype` (new) | **Necessary** | One archetype project is 33.7 KB of text (~8.4k tokens) across `main.tscn`, `project.godot`, `trace.json`, `commands.jsonl` and 3 `GDScript` files. Without a tool, assembling it means pulling all of that through context and re-emitting it - 4-8x above the threshold where transcription loss dominates, and a dropped line is indistinguishable from an intentional edit. It also runs the three gates on what it wrote. `gamecraft-cli forge` is not a substitute: it is unreachable from any client that has no shell. |
| `feel_lint` | **Cut** | Cannot be done reliably by static analysis. "Implements coyote time via a state machine" and "does not implement coyote time" look the same to a regex. A tool that guesses sends agents chasing ghosts. |
| raycast / overlap / distance | **Defer** | No current need. AABB answers the existing questions. Add operations when a concrete task cannot be answered without them, not before. |

Final set: **7 tools, 4 new** (`scene_inspect`, `scene_verify`, `scene_edit`,
`forge_archetype`).

### Coverage audit: every desktop action, and its programmatic path

The fleet property is that no user-visible action is UI-only. Tested against the
desktop shell, action by action:

| Desktop action | Programmatic path | Status |
|---|---|---|
| Select a node in the tree | Tools address nodes by path directly, so selection has no equivalent state to set | covered by representation |
| Drag inspector position/scale | `scene_edit` action `set_transform` | covered |
| Palette "add node" | `scene_edit` action `add_node` | covered |
| Undo / redo | Per-call command stack; cross-call revert is `.bak` restore | covered, one call at a time |
| Click a lint finding | `scene_lint` already returns the node path | covered |
| Orbit the camera and look | `scene_verify(mode="frame")` with an explicit camera | **gap - fixed by this plan** |
| Scrub the trace | `scene_verify(mode="trace")` for numbers, `frame` for a given tick | covered |
| Viewport AABB / collision colours | `spatial_query`, plus the same projection `frame` uses | covered |

Two notes on scope:

- **View state is not scene state.** Camera, selection and scrubber are operator
  console state. They get automatable equivalents by taking them as *arguments*
  (camera in, frame out) rather than by exposing a stateful UI-control channel.
  That satisfies the property without a second API.
- **No action is UI-only.** The three previously covered by nothing were all
  "look at it" variants, which is why one mode closes them all.

### Why the CLI is not a substitute

The MCP surface has to be self-sufficient for a client with file IO and no shell.

| Client | Shell | Can run `gamecraft-cli forge`? |
|---|---|---|
| Claude Code, opencode, Cursor | yes | yes - but it must know the exe path, get args and cwd right, and parse stdout |
| Claude Desktop | no | no |

A capability whose only path is "run this binary" is not exposed, it is
documented. The CLI exists for humans and CI - `just forge`, `just depot-godot`,
`just depot-lint` - and for shell-capable agents that prefer it. It is a second
front door to the same engines, never the only one.

---

## 3. Step zero: break the cycle (two moves)

Two of the new tools need code that sits above `gamecraft-mcp` in the current
graph, and `gamecraft-forge` depends on `gamecraft-mcp` (it calls `scene_lint`
during the depot gate). So `gamecraft-mcp -> gamecraft-forge` is a Cargo cycle.

- `scene_verify(mode="trace")` needs `forge/src/sim.rs`.
- `forge_archetype` needs the builders in `gamecraft-forge`.

Move 1 - create `crates/gamecraft-sim` (L1, std only). Move
`gamecraft-forge/src/sim.rs` verbatim: `TraceInput`, `Phys`, `Platform`, `Body2`,
`step_body`, `run_trace`, `TraceRun`, `bfs_path`, `solve_sokoban`. Both forge and
mcp depend on it.

Move 2 - move `lint_live`, `LintDiagnostic` and the `ERR_*` constants from
`gamecraft-mcp/src/lib.rs` into `gamecraft-godot`, which already owns
`build_graph` and `build_registry`. `gamecraft-mcp` and the desktop re-export
them, so no public API breaks. `gamecraft-forge` then drops its
`gamecraft-mcp` dependency entirely.

Rejected alternative: putting the builders in their own crate. It would be a
third crate to buy the same thing - forge's only reason to depend on mcp is one
lint call, and the lint belongs where the graph is built.

Resulting graph, no cycles:

```
gamecraft-core <- gamecraft-godot <- gamecraft-forge
      ^               ^    ^                ^
      |               |    |                |
gamecraft-commands ---+    |          gamecraft-sim
      ^                    |                ^
      +---- gamecraft-mcp ------------------+
```

`gamecraft-mcp` depends on core, commands, godot, sim, forge. `gamecraft-forge`
depends on core, commands, godot, sim. Nothing mcp needs points back at mcp.

Acceptance: `cargo tree -p gamecraft-forge | rg gamecraft-mcp` returns nothing,
`cargo tree -p gamecraft-mcp | rg gamecraft-forge` shows the edge, and
`cargo run -p gamecraft-cli -- forge --all --out .` still produces 138
byte-identical files.

---

## 4. Contracts

### 4.1 Scene input (all scene-consuming tools)

Exactly one of:

| Field | Meaning |
|---|---|
| `scene_text` | `.tscn` source. Today's behavior, unchanged. |
| `scene_file` | Absolute path, or `res://...` only when `root` is given. |
| `root` | Absolute project directory, used solely to resolve `res://`. |

Both supplied -> error, not a guess. `res://` without `root` -> error naming
`root`. Resolved path escaping `root` -> error. Existing `scene_text`-only calls
keep working.

Rationale for allowing files at all: today an agent holding only these tools
cannot obtain the text to lint in the first place. Requiring a second server for
that is a design hole, not a feature.

### 4.2 Writes (`scene_edit` only)

- `dry_run` **defaults to true**.
- Atomic write: temp file in the same directory, then rename.
- Timestamped `.bak` before the first mutation of a call.
- Response: `{applied, dry_run, changed, path, backup, sha256_before,
  sha256_after}`.
- No write outside `root`.
- Undo stack is per call. There is no cross-call undo, and the tool description
  says so.

---

## 5. Phases

Each phase ends with a test that would fail if the tool were a stub.

### Phase 0 - extract `gamecraft-sim`
Section 3. Nothing else starts until `cargo tree` is clean.

### Phase 1 - `scene_inspect` (read-only)

- `operation`: `tree` | `signals` | `property` | `find`.
- `tree`: names, types, paths, local transforms, script paths, collision
  layer/mask. `depth`, plus `limit`/`cursor` (the maze scene has 151 nodes; an
  unbounded dump is unusable).
- `signals`: connections with `resolved: bool` and, when false, the reason - from
  `SignalRegistry::resolve`, which already exists.
- `property`: one node's properties. `find`: substring over names and types.
- Does not report connection `flags` - the parser does not read them (section 7).

Acceptance: `tree` over all 12 depot scenes returns node counts equal to
`rg -c "^\[node"` per scene (verified: 12/12 agree, 21 to 151 nodes each);
`signals` on `platformer` returns 7 edges, all `resolved: true`.

### Phase 2 - `scene_verify`

- `mode="trace"`: `{archetype, constants?, input_script?, max_frames?}` ->
  `{status, frames_elapsed, terminal_state, assertions}`. Unknown archetype is an
  error listing the supported set.
- `mode="godot"`: `{project_path, frames?}` -> spawn
  `godot --headless --path ... --quit-after N`, hard timeout, return only lines
  matching `ERROR|WARNING|Parse Error` plus a raw tail. Godot resolved from
  `GAMECRAFT_GODOT`, then `PATH` - never a hardcoded machine path.
- `mode="frame"`: `{view?, camera?, tick?}` -> PNG image content plus a JSON
  summary (node count drawn, whether the AABB is empty). Renders the parsed graph
  through the deterministic 320x180 rasterizer: one box per node from its world
  AABB, coloured by type (physics body, mesh, collision shape, plain), red for a
  body that failed the collision-shape rule, yellow for the selection.
  - This is a **schematic**, not a Godot screenshot. Label it as such in the
    response and in the tool description.
  - `mode="shot"` (a real viewport capture) is feasible but costs a rendering
    context. Measured twice:
    - Under `--headless`: `get_viewport().get_texture()` is null, script reports
      `no image`, no PNG written. Headless has no framebuffer.
    - Windowed (`--resolution 200x200`, no `--headless`): 200x200 PNG, sampled
      pixels match the authored colours exactly - clear colour
      `(0.2, 0.6, 0.898)` and a `ColorRect` at `(1.0, 0.349, 0.098)`.
    So `shot` spawns Godot with a visible window for roughly a second, captures,
    and quits. It must stay opt-in (never the default mode) and must document the
    window flash in its description.
  - Default `view` is the scene AABB with a margin, so a caller with no camera
    still gets a usable image.
  - Requires a new `graph -> Vec<Glyph>` projection. Note that the desktop
    viewport currently does an ad-hoc version of this with the `egui` painter;
    the projection should live in one place and both should use it, otherwise the
    two renderings drift.

Acceptance: `trace` on `platformer` reports 414 frames and 4/4 coins, matching the
forge; `godot` on `depot/games/platformer` returns zero error lines, and on a
deliberately corrupted copy returns the parse error; `frame` on `platformer`
returns a non-empty PNG whose drawn-node count matches the scene's node count, and
two calls with identical arguments return byte-identical PNGs.

### Phase 3 - `scene_edit` (write)

- Add `RemoveNodeCommand` to `gamecraft-commands` first, with an undo test
  (removing a subtree must restore it exactly). Rev 1 referenced this command as
  if it existed; it does not.
- Actions: `add_node`, `remove_node`, `reparent`, `set_property`, `set_transform`,
  `connect`, `disconnect`.
- Contract per 4.2.

Acceptance: an edit adding a collision shape takes `scene_lint` from 1 finding to
0; re-running the same edit reports `changed: false`; the `.bak` restores
byte-identically.

### Phase 4 - `forge_archetype` (write)

- `{archetype, target_dir, overrides?, overwrite?}` -> writes `project.godot`,
  `main.tscn`, `scripts/`, `trace.json`, `commands.jsonl`, then runs the same
  three gates the forge enforces: lint 0 findings, trace wins within 600 frames,
  Godot headless silent.
- Reuses the `gamecraft-forge` builders; does not reimplement them.
- Refuses a non-empty `target_dir` unless `overwrite: true`. Same atomic-write,
  `.bak` and sha256 contract as `scene_edit`.
- `overrides` reaches the builder's exported constants (`speed`, `jump_velocity`,
  `gravity`, `fire_interval`, ...), which is the part a file copy cannot do.

Acceptance: forging `pong` into an empty temp dir produces a `main.tscn` that
hashes identically to `depot/games/pong/main.tscn`, and `scene_verify(godot)` on
the result is clean.

---

## 6. Gates

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic`
2. `cargo test --workspace` green, with one test per new tool
3. `cargo fmt --all --check`
4. `cargo check -p gamecraft-core --target wasm32-unknown-unknown`

Plus determinism: anything that serializes must be byte-identical on repeat, and
the hash is returned so the caller can verify without trusting the server.

---

## 7. Out of scope, with reasons

| Item | Reason |
|---|---|
| `feel_lint` | Not reliably decidable statically. Would produce findings that are guesses. |
| Connection `flags` | Parser does not read `flags=`. Either add it to parser and serializer first, or omit the field. An always-zero field that looks like data is worse than no field. |
| Raycast / overlap / distance | No task needs them yet. |
| Real viewport capture (`mode="shot"`) | Feasible, proven windowed (200x200 PNG, exact authored colours). Costs a visible window for ~1s per capture, and is impossible under `--headless` (null viewport texture). Ships as an opt-in mode, never the default. |
| Cross-call undo | Needs session state and invalidation rules. A different design, not a parameter. |
| Second export format | `format` is already a parameter. Overte is an enum value plus a serializer. No stub until then: an unsupported format errors and names what is supported. |

---

## 8. Order

| # | Item | Type | Blocks |
|---|---|---|---|
| 0 | `gamecraft-sim` + lint-move extraction | structural | Phases 2 and 4 |
| 1 | `scene_inspect` | read-only | Phase 3 (agents must see before they edit) |
| 2 | `scene_verify` | read-only | trust in everything else |
| 3 | `scene_edit` | write | - |
| 4 | `forge_archetype` | write | - |

Read-only before write. The two write tools come last because they are the only
ones that can lose work.

---

## Revision log

- **rev 4:** Reinstated `forge_archetype`. Rev 3 cut it on the grounds that the
  CLI already writes archetypes, which is false as a design argument: the CLI is
  unreachable from any client without a shell, and a client that cannot run a
  binary cannot copy a project either without pulling 33.7 KB (~8.4k tokens)
  through context. Added the rule that produced the mistake: the MCP surface must
  be self-sufficient for a client with file IO and no shell. Back to 7 tools, and
  the dependency work back to two moves.
- **rev 3:** Removed a tool-count argument that cited how many servers a
  particular machine registers. That is not a design criterion. Re-derived every
  candidate from substitutability. Kept `scene_edit` but demoted to desirable
  below ~5 KB with a stated threshold. Deferred raycast.
- **rev 2:** Corrected rev 1's factual errors and the forge/mcp cycle.
- **rev 1:** original draft.
