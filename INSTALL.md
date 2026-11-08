# INSTALL — GameCraft on a fresh Windows PC

Two commands. Nothing must be pre-installed except Windows + winget
(present on Windows 10 1809+ and all Windows 11).

```powershell
git clone https://github.com/sandraschi/gamecraft.git
cd gamecraft
just bootstrap
```

`just` itself: `winget install --id Casey.Just --source winget`.
Then `just --list` shows everything. `just bootstrap-check` dry-runs the
provisioner (report only, changes nothing).

## What bootstrap gives you

| Tool | Why players need it | How it arrives |
|---|---|---|
| Godot 4.4 editor | Plays and verifies depot games (`depot/games/*/`) | Downloaded to `.tools/godot`, wired via `GAMECRAFT_GODOT` |
| bun | Serves the web arcade (`just web-serve`, port 11258) | Official install script |
| Node.js LTS | Web gate only (`just web-verify`) | winget `OpenJS.NodeJS.LTS` |
| Rust toolchain | Desktop viewer (`just run-desktop`); first build ~10 min | winget `Rustlang.Rustup`, fallback rustup-init |
| wasm target | Contributor probe (`just check-wasm`) | `rustup target add` |

TRELLIS.2 (photo-to-3D props) is **not** provisioned here: 8 GB weights +
conda/CUDA, GPU owners only. See `docs/TRELLIS2.md`, then `trellis setup`.

## Play (no dev skills needed)

```powershell
just web-serve        # web arcade at http://localhost:11258
just run-desktop      # desktop viewer (builds once, then launches)
```

Godot depot games: open `depot/games/<slug>/project.godot` in the Godot
editor from `.tools/godot` and press Play. Sixteen standalone projects,
each with README + cheat sheet.

Prebuilt Windows x64 binaries ride on the GitHub release
(`gamecraft-desktop`, `gamecraft-cli`, `gamecraft-mcp`, plus the three
Bevy game exes). They are built from the tagged source and expect the
repo around them: drop them into `target/release/` (trio) and
`target/play/` (Bevy exes) of your clone and launch from there, so the
viewer finds `depot/`, `web/` and `bevy/games/` by walking up. Godot,
bun and Node still come from `just bootstrap`.

## Manual steps (when bootstrap itself fails)

1. Install `just`: `winget install --id Casey.Just --source winget`, reopen PowerShell.
2. Rust: `winget install --id Rustlang.Rustup --source winget`, reopen PowerShell, `cargo --version`.
3. Godot: download `Godot_v4.4-stable_win64.exe.zip` from
   `github.com/godotengine/godot/releases` (4.4-stable), unzip to
   `.tools/godot`, set user env `GAMECRAFT_GODOT` to the exe path.
4. Node: `winget install --id OpenJS.NodeJS.LTS --source winget`.
5. bun: `powershell -c "irm bun.sh/install.ps1|iex"`.
6. wasm: `rustup target add wasm32-unknown-unknown`.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `cargo` not recognized right after install | Reopen PowerShell (Rustup updates PATH for new shells) |
| `godot` not found by gates | Set user `GAMECRAFT_GODOT` to the full exe path; reopen shell |
| Port 11258 busy / server will not start | `tailscale serve` holds that tailnet port (remote play) and a 0.0.0.0 bind then fails with EADDRINUSE. The arcade binds 127.0.0.1 by default, which coexists; do not force `HOST=0.0.0.0` on a tailscale-served port. Another looping `bun run serve.ts` (Play-click spawn storm) also holds nothing but litters processes — check `Get-Process bun`. |
| TRELLIS wants Linux | By design: sidecar lives in WSL2, see `docs/TRELLIS2.md` |
| First `run-desktop` takes forever | Normal: Bevy/wgpu compile once (~10 min), then incremental |

Contributors: `just check`, `just test`, `just check-wasm` are the gates.
Repo rules (branches, dirty-tree etiquette, hooks): `AGENTS.md`.
