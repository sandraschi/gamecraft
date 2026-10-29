# Making a new game

Two paths, one compiler. Both land in `depot/games/<slug>/` and both pass the
same gates: `scene_lint` clean, trace completes, Godot boots.

## Which model should I use?

Cloud first, if you have a key. It is faster, needs no GPU, and follows the JSON
schema far more reliably than a mid-size local model:

| Provider | Needs | Command |
|---|---|---|
| OpenAI (or compatible) | `OPENAI_API_KEY` | `--provider openai --model gpt-4o` |
| Anthropic | `ANTHROPIC_API_KEY` | `--provider anthropic` |
| LM Studio | nothing, app running | `--provider auto` or `--endpoint http://127.0.0.1:1234` |
| Ollama | nothing, server running | `--provider local --model <small model>` |

`--provider auto` (the default) probes Ollama on 11434, then LM Studio on 1234, and
uses whichever answers. That exists because "Ollama is down again" is a normal
state, not an exception.

Local reality check, measured on this machine: `gemma4:12b` answers in ~11s but
truncated the spec on one attempt and returned prose on another; `qwen3.8:27b` did
not return a single byte in 420s (it never loaded). A 7B-12B model is workable
**only** because the provider now sends `format: "json"` (Ollama) /
`response_format` (OpenAI-compatible) and the compiler re-asks once with the exact
error. Without those two things, small models are not reliable enough to claim
support for.

## 1. From a prompt (local or cloud model)

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# Local Ollama (default)
cargo run -p gamecraft-cli -- new --prompt "night forest, tricky jumps"

# Cloud (OpenAI-compatible)
$env:OPENAI_API_KEY = "sk-..."
cargo run -p gamecraft-cli -- new --prompt "desert ruins" --provider openai --model gpt-4o

# Anthropic
$env:ANTHROPIC_API_KEY = "sk-ant-..."
cargo run -p gamecraft-cli -- new --prompt "candy castle" --provider anthropic

# LM Studio, a proxy, or a remote Ollama
cargo run -p gamecraft-cli -- new --prompt "ice caverns" --provider openai --endpoint http://127.0.0.1:1234
```

## 2. From a spec file (no model)

```powershell
cargo run -p gamecraft-cli -- new --from-spec tests/fixtures/night_forest_spec.json
```

This is the deterministic path: the spec is the whole input. It is also how the
model path is tested without a model running.

## What the model is allowed to decide

It writes a **spec**, never code:

```json
{
  "slug": "night-forest",
  "title": "Night Forest Run",
  "archetype": "platformer",
  "theme":  { "sky_top": [0.05,0.07,0.2], "ground": [0.18,0.22,0.16], "..." : "..." },
  "physics": { "speed": 5.0, "jump_velocity": 11.0, "gravity": 26.0 },
  "level": {
    "grounds":   [{ "x": -6, "w": 12 }, { "x": 9, "w": 12 }],
    "platforms": [{ "x": 2, "y": 1.6, "w": 2.5 }],
    "coins":     [[2.0, 2.6]],
    "goal_x": 18.0
  }
}
```

`GameCraft` then owns everything that makes it a game rather than a JSON file:
node construction through the command bus, deterministic export, UID hashing,
lint, the trace policy, the autoplay, and the Godot boot check.

## Why not let the model write GDScript

Because then nothing could be asserted. The two most expensive bugs of this
project were a transposed `Transform3D` and a `script` line emitted after the
properties it defines - both passed every load-only check. A model writing raw
scene text would walk straight back into that class of failure, at a volume no
human review can absorb. A spec can be validated, simulated, and repaired
automatically; hand-written scripts cannot.

## Rules the compiler enforces (and reports back to the model)

| Rule | Why |
|---|---|
| `archetype` must be `platformer` | other archetypes are not parameterised yet |
| `slug` is `[a-z0-9-]+` | it becomes a folder name |
| ground slabs need positive width | a zero-width slab is invisible and unlandable |
| `platform.y - 0.2 >= 1.1` | the player is 1.0 tall; a lower platform is a wall the autopilot cannot see |
| the level must be completable | the gap-detecting policy is run in Rust first; failure returns the exact reason and the furthest x reached |

On failure the model gets that reason and one retry. More retries mostly buy
longer failures.

## Honest limits

- **One archetype.** "New game" means a new level, theme and physics feel - not a
  new mechanic. The other twelve depot genres are not parameterised.
- **The model path is verified against a mock server**, not a live model: no
  Ollama was running on this machine, so the HTTP contract was exercised with a
  local stub returning a fixed spec. The provider code for Ollama, OpenAI and
  Anthropic compiles and shares one code path, but only the OpenAI-compatible
  request shape has been run end to end.
- **`curl.exe` is a subprocess**, chosen to keep the forge dependency-free. If it
  is missing, the error names it.
- **No textures yet** on generated games; the wall materials are flat albedo.
