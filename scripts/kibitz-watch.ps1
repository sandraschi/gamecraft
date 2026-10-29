#Requires -Version 7
<#
.SYNOPSIS
  KIBITZ dead-drop watcher: zero-token gate, kicks a headless builder on new entries.

.DESCRIPTION
  One poll pass, designed for a Scheduled Task (see install-kibitz-watch.ps1).
  Reads KIBITZ.md entry headers (lines starting with '## ') and compares the
  newest against scripts/.kibitz-watch.state (one line: last handled header).
  - Same header  -> SKIP, exit 0, zero tokens spent.
  - New header   -> KICK: runs 'opencode run' with the bounded brief and logs
    to scripts/logs/. State advances only when opencode itself launched cleanly
    (agent-internal failures must land in KIBITZ_BACKTALK.md per the brief).
  Entries appended without a '## ' header are invisible to this gate (protocol
  requires headers; noted, not fixed).

.PARAMETER DryRun
  Print SKIP/KICK and the reason without invoking opencode.
#>
param(
    [string]$RepoRoot = "D:\Dev\repos\gamecraft",
    [string]$Opencode = "C:\Users\sandr\scoop\shims\opencode.exe",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$ScriptsDir = Join-Path $RepoRoot "scripts"
$KibitzFile = Join-Path $RepoRoot "KIBITZ.md"
$PromptFile = Join-Path $ScriptsDir "kibitz-builder-prompt.md"
$StateFile = Join-Path $ScriptsDir ".kibitz-watch.state"
$LogDir = Join-Path $ScriptsDir "logs"

foreach ($path in @($KibitzFile, $PromptFile)) {
    if (-not (Test-Path -LiteralPath $path)) {
        Write-Host "missing required file: $path"
        exit 1
    }
}
if (-not (Test-Path -LiteralPath $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir | Out-Null
}

$headers = @(Get-Content -Path $KibitzFile | Where-Object { $_ -match '^## ' })
if ($headers.Count -eq 0) {
    Write-Host "SKIP: no entry headers in KIBITZ.md"
    exit 0
}
$latest = $headers[$headers.Count - 1]

$lastHandled = ""
if (Test-Path -LiteralPath $StateFile) {
    $lastHandled = (Get-Content -Path $StateFile -Raw).Trim()
}

if ($latest -eq $lastHandled) {
    Write-Host "SKIP: latest entry already handled ($latest)"
    exit 0
}

if ($DryRun) {
    Write-Host "KICK (dry run): new entry: $latest"
    exit 0
}

if (-not (Test-Path -LiteralPath $Opencode)) {
    Write-Host "opencode not found: $Opencode"
    exit 1
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$logFile = Join-Path $LogDir "kibitz-watch-$stamp.log"
$prompt = (Get-Content -Path $PromptFile -Raw).Trim()
& $Opencode run --dir $RepoRoot --title "kibitz-watch $stamp" "$prompt" *> $logFile
if ($LASTEXITCODE -ne 0) {
    Write-Host "opencode exited with code $LASTEXITCODE, log: $logFile (state NOT advanced, will retry next poll)"
    exit 1
}

Set-Content -Path $StateFile -Value $latest -NoNewline
Write-Host "KICK done, log: $logFile"
exit 0
