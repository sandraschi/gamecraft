# GameCraft launcher: builds (when stale) and runs the desktop viewer.
#
# Double-click start.bat, or run this directly:
#   powershell.exe -NoProfile -ExecutionPolicy Bypass -File start.ps1
$ErrorActionPreference = "Stop"

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: cargo not found. Install Rust: https://rustup.rs" -ForegroundColor Red
    exit 1
}

Set-Location -LiteralPath $PSScriptRoot
& cargo run -q -p gamecraft-desktop
