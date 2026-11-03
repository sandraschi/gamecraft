# TRELLIS.2 integration

Thin sidecar, never a bundle. TRELLIS.2 is a 4B-parameter Python/CUDA
image-to-3D model (`github.com/microsoft/TRELLIS.2`, paper
`arxiv:2512.14692`); it cannot live in a Pure-Rust workspace and is not
vendored here.

## Licence (read first)

TRELLIS.2 is **research-only (non-commercial)** per its project page. Every
plan writes a ledger row whose licence field says so and whose status is
`pending-human-approval`. No TRELLIS-derived asset ships in `depot/` or a
release without a human sign-off recorded in the ledger. See
`LLM_AS_GAME_DESIGNER.md` section 7 for the ledger rules this plugs into.

## Architecture

| Piece | Role |
|---|---|
| `crates/gamecraft-trellis` | L1: `TrellisConfig` (env/flags), `plan()` (deterministic argv + ledger), `probe()` (presence check, no spawning), `check_poly_budget` / `check_extent` / `ledger_complete` gates, `write_ledger`. No torch, no network in tests. |
| `gamecraft-cli trellis` | L3: dry-run plan by default; `--execute` spawns exactly one sidecar process with a 60 s timeout and kills it on expiry. |
| `asset.trellis` | MCP/registry command (`tool_trellis_asset`): plan-only, same code path as the CLI dry run. Execution is never available behind MCP. |

TRELLIS.2 is **image-to-3D** (TRELLIS.1 took text; .2 takes an image).
`prompt_ref` is ledger context only, never a generation input. Text-to-3D
means text-to-image first (an image model), then this pipe.

## Setup (operator-owned)

```powershell
git clone https://github.com/microsoft/TRELLIS.2 D:/Tools/TRELLIS.2
# create the env per its README (CUDA, ~24 GB VRAM; Goliath's 4090 qualifies)
$env:GAMECRAFT_TRELLIS_REPO = "D:/Tools/TRELLIS.2"
$env:GAMECRAFT_TRELLIS_PYTHON = "D:/Tools/TRELLIS.2/.venv/Scripts/python.exe"
# or HTTP: $env:GAMECRAFT_TRELLIS_BACKEND = "http"
#           $env:GAMECRAFT_TRELLIS_ENDPOINT = "http://127.0.0.1:7860"
```

## Use

```powershell
# plan only: prints argv, out path, licence; writes nothing
cargo run -p gamecraft-cli -- trellis --image shots/farmhouse.png --slug farmhouse-hero
# run it (60 s timeout, ledger written next to the GLB)
cargo run -p gamecraft-cli -- trellis --image shots/farmhouse.png --slug farmhouse-hero --execute
```

Import gate for the resulting GLB: `check_poly_budget` (default 20 000 tris),
`check_extent` (default 8 m), plus the existing style-bible/palette lint from
`LLM_AS_GAME_DESIGNER.md` section 5.4 before the asset enters `web/` or the depot.
