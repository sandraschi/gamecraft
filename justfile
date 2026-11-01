set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# GameCraft automation (Pure-Rust ArtCraft pattern).
# Every recipe prefixes cargo via USERPROFILE so NSSM / IDE shells work
# even when .cargo/bin is not on the inherited PATH.

default:
    @just --list

# Strict ArtCraft lint gate across all workspace members
check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit + integration tests
test:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo test --workspace

# Formatting gate
fmt-check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo fmt --all --check

# wasm portability probe for gamecraft-core (no wgpu/egui/godot deps allowed)
check-wasm:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo check -p gamecraft-core --target wasm32-unknown-unknown

# Run the MCP server locally over stdio (logs on stderr, JSON-RPC on stdout)
run-mcp:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; $env:RUST_LOG = "debug"; cargo run -p gamecraft-mcp

# CLI smoke: lint the bundled sample scene
run-cli:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p gamecraft-cli -- lint --scene apps/gamecraft-cli/sample.tscn

# Full static gate: clippy + fmt check + wasm probe (spec: just check)
check-all:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo check --workspace --all-targets
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo fmt --all -- --check
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic

# Spec compat: strict clippy with pedantic
clippy:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic

# Spec compat: format codebase
fmt:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo fmt --all

# Launch desktop viewer (debug)
run-desktop:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run --bin gamecraft-desktop

# Headless verification against sample Godot scenes (spec: smoke-godot)
smoke-godot:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo test -p gamecraft-godot --test scene_parsing_smoke -- --nocapture
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p gamecraft-cli -- lint --scene tests/fixtures/sample_3d_world.tscn

# Production build (CLI + MCP + desktop binaries)
build-release:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo build --release --workspace

# Regenerate the 15-game depot (deterministic: same bytes every run)
forge:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p gamecraft-cli -- forge --all --out .

# Gate: every depot scene must lint clean (all 15, not three samples)
depot-lint:
    pwsh -NoProfile -Command "$env:Path = ""$env:USERPROFILE\.cargo\bin;$env:Path""; $fail=0; Get-ChildItem depot/games/*/main.tscn | ForEach-Object { $r = cargo run -q -p gamecraft-cli -- lint --scene $_.FullName 2>&1 | Out-String; if ($r -match '"findings":\s*\[\s*\]') { Write-Host ""$($_.Directory.Name): clean"" } else { $fail++; Write-Host ""$($_.Directory.Name): FINDINGS"" } }; if ($fail -gt 0) { exit 1 }"

# Gate: all 15 depot projects must boot in Godot with zero ERROR/WARNING lines
depot-godot:
    pwsh -NoProfile -Command "$g=(Get-Command godot).Source; Get-ChildItem depot/games | ForEach-Object { $o = & $g --headless --path $_.FullName --quit-after 240 2>&1 | Out-String; $bad = ($o -split \"`n\" | Where-Object { $_ -match 'ERROR|WARNING|Parse Error' }); if ($bad) { Write-Host \"$($_.Name): $bad\" } else { Write-Host \"$($_.Name): clean\" } }"

# Gate: every depot game must PRINT the win marker in Godot, not merely boot.
# depot-godot proves a scene loads; this proves it can be won. They are different
# claims and only this one is what "verified" means. Frame budget is 3600 (~60 s
# of simulated play at 60 fps), which is generous for the current run lengths.
verify-depot:
    pwsh -NoProfile -File scripts/verify-depot.ps1

# Godot asset import pass. Required once after forging any game that ships
# generated textures or models: headless --import is broken in 4.4 (progress
# dialog errors, no .import files), an editor pass works.
godot-import:
    pwsh -NoProfile -Command "$g=(Get-Command godot).Source; Get-ChildItem depot/games | ForEach-Object { & $g --editor --path $_.FullName --quit-after 120 | Out-Null }; Write-Host 'imports written'"

# three.js web family: serve the arcade on 11258
web-serve:
    $env:Path = "$env:USERPROFILE\.bun\bin;$env:Path"; Set-Location web; bun run serve

# three.js web family gate: headless boot + self-play win + screenshots (Node 24)
web-verify:
    $env:Path = "$env:USERPROFILE\.bun\bin;$env:Path"; Set-Location web; node scripts/verify.ts

# PWA gate: manifests + icons, service worker precache, every game boots offline
web-pwa:
    $env:Path = "$env:USERPROFILE\\.bun\\bin;$env:Path"; Set-Location web; node scripts/pwa-check.ts

# glTF gate: export every depot scene, Khronos-validate (0 errors), render each in three.js
gltf-gate:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:USERPROFILE\\.bun\\bin;$env:Path"; cargo run -q -p gamecraft-cli -- gltf --depot-out target/gltf; if ($LASTEXITCODE) { exit 1 }; Set-Location web; node --no-deprecation scripts/gltf-validate.ts ../target/gltf; if ($LASTEXITCODE) { exit 1 }; node --no-deprecation scripts/gltf-shot.ts ../target/gltf/shots (Get-ChildItem ../target/gltf/*.glb | ForEach-Object FullName)

# Bevy gate: headless logic tests (autoplay wins on the shortest path), then the real
# windowed build plays itself to the win marker and saves target/bevy/maze.png
bevy-gate:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo test -q -p gamecraft-bevy; if ($LASTEXITCODE) { exit 1 }; cargo build -q -p gamecraft-bevy --features render --bin gamecraft-bevy-maze; if ($LASTEXITCODE) { exit 1 }; New-Item -ItemType Directory -Force target/bevy | Out-Null; $out = & ./target/debug/gamecraft-bevy-maze.exe --autoplay --exit-on-win --screenshot target/bevy/maze.png 2>$null | Out-String; Write-Host $out.Trim(); if ($out -notmatch 'goal reached' -or -not (Test-Path target/bevy/maze.png)) { Write-Host 'bevy maze: no win marker or no screenshot'; exit 1 }; Add-Type -AssemblyName System.Drawing; $bmp = [System.Drawing.Bitmap]::FromFile((Resolve-Path target/bevy/maze.png)); $bg = $bmp.GetPixel(0,0); $clip = 0; foreach ($x in 0..($bmp.Width-1)) { foreach ($y in 0,1,($bmp.Height-2),($bmp.Height-1)) { if ($bmp.GetPixel($x,$y) -ne $bg) { $clip++ } } }; foreach ($y in 0..($bmp.Height-1)) { foreach ($x in 0,1,($bmp.Width-2),($bmp.Width-1)) { if ($bmp.GetPixel($x,$y) -ne $bg) { $clip++ } } }; $bmp.Dispose(); if ($clip) { Write-Host "bevy maze: clipped at the screenshot border ($clip px)"; exit 1 }; Write-Host 'bevy maze: PASS'
