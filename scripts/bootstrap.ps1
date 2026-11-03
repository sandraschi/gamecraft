#Requires -Version 5.1
<#
.SYNOPSIS
  Naked-PC bootstrap for GameCraft: provision cargo, Godot 4.4, Node, bun,
  and the wasm target on a stock Windows machine. Idempotent: present tools
  are verified, never reinstalled.
.DESCRIPTION
  -Check dry-runs (report only, change nothing). Without -Check, missing
  tools are installed via winget or official downloads, then verified.
  Fails loud in the first seconds when something cannot be provided.
  Player order: the script provisions play dependencies first (Godot for
  depot games, bun for the web arcade), dev tooling second.
#>
[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$Yes
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$ToolsDir = Join-Path $RepoRoot ".tools"

# Session-only PATH for well-known per-user install dirs (cargo, bun, local
# shims). Mirrors the justfile prefixing; system PATH is never touched here.
foreach ($dir in @("$env:USERPROFILE\.cargo\bin", "$env:USERPROFILE\.bun\bin", "$env:USERPROFILE\.local\bin")) {
    if ((Test-Path $dir) -and ($env:Path -notlike "*$dir*")) { $env:Path = "$dir;$env:Path" }
}

function Write-Step([string]$Text) { Write-Host "== $Text" -ForegroundColor Cyan }
function Write-Ok([string]$Text) { Write-Host "  [ok] $Text" -ForegroundColor Green }
function Write-Miss([string]$Text) { Write-Host "  [missing] $Text" -ForegroundColor Yellow }
function Write-Bad([string]$Text) { Write-Host "  [FAIL] $Text" -ForegroundColor Red }

function Test-Cmd([string]$Cmd) {
    return $null -ne (Get-Command $Cmd -ErrorAction SilentlyContinue)
}

function Install-Winget([string]$WingetId, [string]$Label) {
    if (-not (Test-Cmd "winget")) {
        Write-Bad "winget unavailable; install $Label manually ($WingetId)."
        exit 1
    }
    Write-Host "  installing $Label via winget ($WingetId) ..." -ForegroundColor Yellow
    & winget install --id $WingetId --source winget --silent `
        --accept-source-agreements --accept-package-agreements --disable-interactivity
    $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + `
                [System.Environment]::GetEnvironmentVariable("PATH", "User")
}

function Get-GodotExe {
    $fromEnv = $env:GAMECRAFT_GODOT
    if ($fromEnv -and (Test-Path $fromEnv)) { return $fromEnv }
    $g = Get-Command "godot" -ErrorAction SilentlyContinue
    if ($g) { return $g.Source }
    $local = Join-Path $ToolsDir "godot\Godot_v4.4-stable_win64.exe"
    if (Test-Path $local) { return $local }
    return $null
}

$failures = 0

# --- 1. Rust toolchain (players need it for the desktop viewer; contributors for everything)
Write-Step "Rust toolchain (cargo)"
if (Test-Cmd "cargo") {
    Write-Ok ((& cargo --version 2>$null))
} else {
    Write-Miss "cargo not on PATH"
    if (-not $Check) {
        Install-Winget "Rustlang.Rustup" "Rustup"
        if (-not (Test-Cmd "cargo")) {
            Write-Host "  winget Rustup failed; falling back to rustup-init.exe ..." -ForegroundColor Yellow
            $init = Join-Path $env:TEMP "rustup-init.exe"
            Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $init
            & $init -y --default-toolchain stable --profile minimal | Out-Null
            $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + `
                        [System.Environment]::GetEnvironmentVariable("PATH", "User")
            Remove-Item $init -Force -ErrorAction SilentlyContinue
        }
    }
}
if (Test-Cmd "cargo") { Write-Ok ((& cargo --version 2>$null)) }
else { Write-Bad "cargo still missing (reopen PowerShell after Rustup, then retry)"; $failures++ }

# --- 2. Godot 4.4 editor (plays and verifies depot games; GAMECRAFT_GODOT wins over PATH)
Write-Step "Godot 4.4 editor"
$godot = Get-GodotExe
if ($godot) {
    Write-Ok ("Godot at $godot")
} else {
    Write-Miss "Godot 4.4 not found"
    if (-not $Check) {
        $zip = Join-Path $env:TEMP "godot-4.4.zip"
        $url = "https://github.com/godotengine/godot/releases/download/4.4-stable/Godot_v4.4-stable_win64.exe.zip"
        Write-Host "  downloading Godot 4.4 (~100 MB) ..." -ForegroundColor Yellow
        New-Item -ItemType Directory -Force (Join-Path $ToolsDir "godot") | Out-Null
        Invoke-WebRequest -Uri $url -OutFile $zip
        Expand-Archive -Path $zip -DestinationPath (Join-Path $ToolsDir "godot") -Force
        Remove-Item $zip -Force -ErrorAction SilentlyContinue
        $godot = Get-GodotExe
    }
}
if (Get-GodotExe) {
    $g = Get-GodotExe
    Write-Ok ("Godot at $g")
    if ((-not $env:GAMECRAFT_GODOT) -and (-not $Check)) {
        [System.Environment]::SetEnvironmentVariable("GAMECRAFT_GODOT", $g, "User")
        $env:GAMECRAFT_GODOT = $g
        Write-Host "  set user GAMECRAFT_GODOT=$g (reopen shells to inherit)" -ForegroundColor Yellow
    }
} else { Write-Bad "Godot 4.4 unavailable"; $failures++ }

# --- 3. Node.js LTS (web-verify gate; developers)
Write-Step "Node.js LTS"
if (Test-Cmd "node") {
    Write-Ok ((& node --version 2>$null))
} else {
    Write-Miss "node not on PATH"
    if (-not $Check) { Install-Winget "OpenJS.NodeJS.LTS" "Node.js LTS" }
}
if (Test-Cmd "node") { Write-Ok ((& node --version 2>$null)) }
else { Write-Bad "node still missing (reopen PowerShell, then retry)"; $failures++ }

# --- 4. bun (serves the web arcade: just web-serve)
Write-Step "bun (web arcade server)"
$bunLocal = Join-Path $env:USERPROFILE ".bun\bin\bun.exe"
if ((Test-Path $bunLocal) -or (Test-Cmd "bun")) {
    if (Test-Cmd "bun") { Write-Ok ((& bun --version 2>$null)) }
    else { Write-Ok "bun at $bunLocal" }
} else {
    Write-Miss "bun not found"
    if (-not $Check) {
        Write-Host "  installing bun (official install.ps1) ..." -ForegroundColor Yellow
        Invoke-Expression ((New-Object System.Net.WebClient).DownloadString("https://bun.sh/install.ps1"))
        $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + `
                    [System.Environment]::GetEnvironmentVariable("PATH", "User")
    }
}
if ((Test-Path $bunLocal) -or (Test-Cmd "bun")) { Write-Ok "bun present" }
else { Write-Bad "bun still missing"; $failures++ }

# --- 5. wasm target (check-wasm probe; contributors)
Write-Step "wasm32-unknown-unknown target"
if (Test-Cmd "rustup") {
    $installed = (& rustup target list --installed 2>$null | Out-String)
    if ($installed -match "wasm32-unknown-unknown") {
        Write-Ok "wasm32-unknown-unknown installed"
    } else {
        Write-Miss "wasm target not installed"
        if (-not $Check) { & rustup target add wasm32-unknown-unknown }
    }
} else {
    Write-Bad "rustup missing; cannot verify wasm target"; $failures++
}

# --- 6. GPU sidecar note (TRELLIS; NOT installed here)
Write-Step "TRELLIS.2 sidecar (optional, GPU owners only)"
Write-Host "  not provisioned by bootstrap: 8 GB weights + conda/CUDA env." -ForegroundColor Yellow
Write-Host "  players who want photo-to-prop: see docs/TRELLIS2.md, then trellis setup (job 2)."

Write-Host ""
if ($Check) {
    Write-Host "bootstrap -Check: dry run, nothing changed." -ForegroundColor Cyan
    exit 0
}
if ($failures -gt 0) {
    Write-Bad "$failures tool(s) still missing; see lines above, fix, re-run."
    exit 1
}
Write-Host "bootstrap complete: play with 'just web-serve' (arcade), 'just run-desktop' (viewer)," -ForegroundColor Green
Write-Host "  or open a depot game in the Godot editor at $ToolsDir\godot and press Play." -ForegroundColor Green
