# TRELLIS.2 integration

Thin sidecar, never a bundle. TRELLIS.2 is a 4B-parameter Python/CUDA
image-to-3D model (`github.com/microsoft/TRELLIS.2`, paper
`arxiv:2512.14692`, HF weights `microsoft/TRELLIS.2-4B`, demo
`huggingface.co/spaces/microsoft/TRELLIS.2`); it cannot live in a
Pure-Rust workspace and is not vendored here.

Roadmap status 2026-11-07: complete — paper, image-to-3D inference,
4B checkpoints, Spaces demo, shape-conditioned texture generation
(`example_texturing.py`, `app_texturing.py`), and training code are
all released upstream.

## Is it good for props from photos? Yes — that is the core use case

The upstream minimal example is exactly this pipe: one input photo goes
into `Trellis2ImageTo3DPipeline`, a mesh comes out, `to_glb(...)` writes a
PBR-ready `sample.glb`. Photo in, game prop out. This is what
`gamecraft-cli trellis` wraps.

Honest limits (all from the upstream README, re-verified 2026-11-07):

- **Single view in, hallucinated back.** One photo constrains the front;
  the model invents the unseen sides. Front-facing hero props and
  background clutter: great. Assets the player orbits closely: inspect
  the back before accepting.
- **Topology still needs cleanup.** The paper's headline result is
  arbitrary topology (open, non-manifold, enclosed) with sharp features —
  a real step past iso-surface methods — but the fleet's standing rule
  from `LLM_AS_GAME_DESIGNER.md` §5.2 holds: poly budget, UVs, pivots and
  a cohesion pass against the style bible before anything enters `web/`.
- **Default output is enormous.** Upstream's GLB export defaults to
  `decimation_target=1000000` triangles and `texture_size=4096`. That is
  why this crate's gates exist: `check_poly_budget` (default 20 000),
  `check_extent` (default 8 m), texture downscale on import.
- **Alpha is preserved but off.** The GLB exports `OPAQUE` by default;
  transparency needs the alpha channel wired to opacity manually in the
  DCC. Glass, foliage cards: plan that step.
- **Remesh is on by default** (`remesh=True` upstream): clean UVs out,
  some sharp-feature softening in. Match it against the style bible.

## v1 vs v2: much better at image-to-3D, not a strict superset

| | TRELLIS (v1) | TRELLIS.2 |
|---|---|---|
| Params | 2B | 4B |
| Inputs | text **or** image | **image only** (+ shape-conditioned PBR texturing as a separate path) |
| Representation | SLAT (iso-surface field limits) | O-Voxel: field-free, open / non-manifold / enclosed |
| Materials | texture colour | full PBR: base colour, roughness, metallic, **opacity** |
| Compression | — | sparse VAE, 16x spatial downsampling |
| Resolution | — | up to 1536 cube |
| Speed (H100) | — | ~3 s @512, ~17 s @1024, ~60 s @1536 (shape+material) |
| Training data | 500K curated objects | diverse public 3D sets, unlisted |

So "much better" is fair for geometry, materials and topology — the
paper's claim ("far exceed those of existing models") is about exactly
that — but direct **text-to-3D is gone** in v2. Text goes text-to-image
first (an image model), then this pipe. `prompt_ref` stays ledger
context only, never a generation input, for exactly this reason.

## Licence (read first)

Nuanced, verified against the repo README (2026-11-03):

- **Code + model: MIT** (`LICENSE` in the repo).
- **Project page framing: research-only**, non-commercial intent for the
  published materials.
- **Training data: unlisted public 3D scrapes** ("diverse public 3D
  asset datasets", bias explicitly not ruled out).
- **NVIDIA deps carry their own licences** (`nvdiffrast`, `nvdiffrec`).

Fleet rule §7 (no scraped assets without a clear licence) therefore
still bites on the *provenance* side: every plan writes a ledger row
whose licence field states the MIT/provenance-unclear split and whose
status is `pending-human-approval`. No TRELLIS-derived asset ships in
`depot/` or a release without a recorded human sign-off. Generated
content must also be disclosed where stores require it (Steam asks).

## Architecture

| Piece | Role |
|---|---|
| `crates/gamecraft-trellis` | L1: `TrellisConfig` (env/flags), `plan()` (deterministic argv + ledger), `probe()` (presence check, no spawning), `check_poly_budget` / `check_extent` / `ledger_complete` gates, `write_ledger`. No torch, no network in tests. |
| `gamecraft-cli trellis` | L3: dry-run plan by default; `--execute` spawns exactly one sidecar process with a 60 s timeout and kills it on expiry. |
| `asset.trellis` | MCP/registry command (`tool_trellis_asset`): plan-only, same code path as the CLI dry run. Execution is never available behind MCP. |

## Setup (operator-owned, Linux)

Upstream is **tested on Linux only**: conda env, CUDA 12.4, NVIDIA GPU
with **24+ GB VRAM**. Env default is Python 3.8+ with torch 2.6.0 /
CUDA 12.4 and the `flash-attn` backend; GPUs without flash-attn
support (e.g. V100, not our 4090) fall back via `ATTN_BACKEND=xformers`
plus a manual xformers install. Upstream also ships `app.py` (image
to 3D) and `app_texturing.py` (shape-conditioned PBR) web demos —
handy for eyeballing a prop before wiring the pipe.
Goliath's 4090 clears the VRAM bar but runs
Windows 11 — the sidecar belongs in **WSL2** (or a Linux box), with
`GAMECRAFT_TRELLIS_REPO` / `GAMECRAFT_TRELLIS_PYTHON` pointing at it.
H100 timings above are upstream's; a 4090 will be slower per step, and
1536-cube on 24 GB is the tightest fit — start at 1024.

```powershell
# plan only: clone, conda env, weights (~8 GB), driver — prints every step, runs nothing
cargo run -p gamecraft-cli -- trellis-setup
# env + driver only, skip the weight download
cargo run -p gamecraft-cli -- trellis-setup --skip-weights
# really provision (long steps, 30 min cap each, killed on expiry)
cargo run -p gamecraft-cli -- trellis-setup --execute
```

`trellis setup` is the one-click: it clones the repo, builds the conda
env with upstream's own `setup.sh` flags, pulls `microsoft/TRELLIS.2-4B`,
and writes the driver below as `<repo>/trellis_driver.py` from the
embedded `gamecraft_trellis::DRIVER_SOURCE` const (single source of
truth — the copy in this doc is reference only). Steps whose markers
exist are skipped; `--force` re-runs them.

Manual fallback (same steps by hand):

```powershell
git clone https://github.com/microsoft/TRELLIS.2 D:/Tools/TRELLIS.2 --recursive
# create the env per its README: . ./setup.sh --new-env --basic --flash-attn --nvdiffrast --nvdiffrec --cumesh --o-voxel --flexgemm
$env:GAMECRAFT_TRELLIS_REPO = "D:/Tools/TRELLIS.2"
$env:GAMECRAFT_TRELLIS_PYTHON = "D:/Tools/TRELLIS.2/.venv/Scripts/python.exe"
# or HTTP: $env:GAMECRAFT_TRELLIS_BACKEND = "http"
#           $env:GAMECRAFT_TRELLIS_ENDPOINT = "http://127.0.0.1:7860"
```

```python
"""trellis_driver.py: image -> GLB, game-budget defaults. Adapted from
upstream example.py (MIT). Reference copy of gamecraft_trellis::DRIVER_SOURCE;
trellis setup writes the live one — edit the const, not this block."""
import argparse
import os

os.environ["OPENCV_IO_ENABLE_OPENEXR"] = "1"
os.environ["PYTORCH_CUDA_ALLOC_CONF"] = "expandable_segments:True"

import o_voxel  # noqa: E402
from trellis2.pipelines import Trellis2ImageTo3DPipeline  # noqa: E402
from PIL import Image  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("--image", required=True)
ap.add_argument("--outdir", required=True)
ap.add_argument("--slug", default="asset")
ap.add_argument("--decimation-target", type=int, default=20000)
ap.add_argument("--texture-size", type=int, default=1024)
args = ap.parse_args()

pipe = Trellis2ImageTo3DPipeline.from_pretrained("microsoft/TRELLIS.2-4B")
pipe.cuda()
mesh = pipe.run(Image.open(args.image))[0]
os.makedirs(args.outdir, exist_ok=True)
out = os.path.join(args.outdir, args.slug + ".glb")
glb = o_voxel.postprocess.to_glb(
    vertices=mesh.vertices,
    faces=mesh.faces,
    attr_volume=mesh.attrs,
    coords=mesh.coords,
    attr_layout=mesh.layout,
    voxel_size=mesh.voxel_size,
    aabb=[[-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]],
    decimation_target=args.decimation_target,
    texture_size=args.texture_size,
    remesh=True,
)
glb.export(out)
print("wrote " + out)
```

Note the driver already applies game-budget defaults (20K tris, 1K
textures) so the GLB arrives near the import gates instead of at
upstream's 1M/4K showcase defaults. Parity notes vs the upstream
example (checked 2026-11-07, behaviour unchanged): upstream calls
`mesh.simplify(16777216)` (nvdiffrast cap) for its turntable-render
path and exports with `extension_webp=True` — the game driver skips
both (no turntable render; webp left to the importer).
`remesh_band=1` / `remesh_project=0` match upstream.

## Use

```powershell
# plan only: prints argv, out path, licence; writes nothing
cargo run -p gamecraft-cli -- trellis --image shots/farmhouse.png --slug farmhouse-hero
# run it (60 s timeout, ledger written next to the GLB)
cargo run -p gamecraft-cli -- trellis --image shots/farmhouse.png --slug farmhouse-hero --execute
```

Import gate for the resulting GLB: `check_poly_budget` (default 20 000
tris), `check_extent` (default 8 m), texture-size check against the
1K driver default, plus the style-bible/palette lint from
`LLM_AS_GAME_DESIGNER.md` §5.4 before the asset enters `web/` or the depot.
