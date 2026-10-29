#Requires -Version 7
<#
.SYNOPSIS
  Register (or remove) the GameCraft kibitz-watch Scheduled Task.

.DESCRIPTION
  Mirrors git-github-mcp/scripts/install_morning_task.ps1. Registers a daily
  task that re-runs scripts/kibitz-watch.ps1 every IntervalMin minutes for a
  full day. The watcher itself spends zero tokens when there is nothing new;
  a headless 'opencode run' only fires on unhandled KIBITZ.md entries.

  Goliath is 24/7, so no wake flags. Task runs in Sandra's interactive
  session (locked is fine); opencode uses her existing auth/config.
  Trigger is built with schtasks.exe (daily + repetition, which the
  ScheduledTasks module cannot express with -Daily); the 45-min execution cap
  and no-parallel-runs policy are patched onto the task settings afterwards.

.PARAMETER IntervalMin
  Poll cadence in minutes. Default 30.

.PARAMETER Remove
  Unregister the task instead.
#>
param(
    [int]$IntervalMin = 30,
    [switch]$Remove
)

$ErrorActionPreference = "Stop"
$TaskName = "GameCraft-Kibitz-Watch"
$RepoRoot = "D:\Dev\repos\gamecraft"
$Pwsh = "C:\Program Files\PowerShell\7\pwsh.exe"
$Runner = Join-Path $RepoRoot "scripts\kibitz-watch.ps1"
$Opencode = "C:\Users\sandr\scoop\shims\opencode.exe"

if ($Remove) {
    & schtasks.exe /Delete /TN $TaskName /F
    Write-Host "Removed scheduled task: $TaskName"
    exit 0
}

foreach ($path in @($Pwsh, $Runner, $Opencode)) {
    if (-not (Test-Path -LiteralPath $path)) {
        Write-Host "missing required path: $path"
        exit 1
    }
}

$actionCmd = "`"$Pwsh`" -NoProfile -ExecutionPolicy Bypass -File `"$Runner`" -RepoRoot `"$RepoRoot`""
& schtasks.exe /Create /TN $TaskName /TR $actionCmd /SC DAILY /ST 00:00 /RI $IntervalMin /DU 23:30 /IT /RL LIMITED /F
if ($LASTEXITCODE -ne 0) {
    Write-Host "schtasks /Create failed (exit $LASTEXITCODE). Re-run from an elevated shell, then retry."
    exit 1
}

try {
    $current = Get-ScheduledTask -TaskName $TaskName
    $capped = $current.Settings
    $capped.ExecutionTimeLimit = "PT45M"
    $capped.MultipleInstances = 2
    Set-ScheduledTask -TaskName $TaskName -Settings $capped | Out-Null
    Write-Host "Settings patched: 45 min execution cap, parallel runs ignored."
} catch {
    Write-Host "WARNING: settings patch failed, task runs uncapped: $_"
}

Write-Host "Installed task '$TaskName': daily from 00:00, every $IntervalMin min for 23.5h, interactive-only, least privilege."
Write-Host "  Runner: $Runner"
Write-Host "  Kicking one immediate poll to verify the task path (expect SKIP, state is seeded):"
& schtasks.exe /Run /TN $TaskName
Write-Host "  Remove: install-kibitz-watch.ps1 -Remove"
