# KIBITZ-poller: dead-drop notifier for the two-chat GameCraft loop.
# It does NOT build, does NOT call LLMs, does NOT spend tokens.
# It watches KIBITZ.md (kibitzer to builder) and KIBITZ_BACKTALK.md
# (builder to kibitzer) and prints new tails so the human can relay
# one paste-block per turn. Human stays router; limiters cap the day.
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File KIBITZ-poller.ps1 -IntervalSec 120 -MaxPolls 120
# Stop: Ctrl+C. Quiet hours: 22:00-07:00 Vienna by default (exits).

param(
    [int]$IntervalSec = 120,
    [int]$MaxPolls = 120,
    [int]$TailLines = 25,
    [string]$QuietStart = "22:00",
    [string]$QuietEnd = "07:00"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$kibitz = Join-Path $root "KIBITZ.md"
$backtalk = Join-Path $root "KIBITZ_BACKTALK.md"

function Get-ViennaNow {
    $tz = [System.TimeZoneInfo]::FindSystemTimeZoneById("W. Europe Standard Time")
    return [System.TimeZoneInfo]::ConvertTimeFromUtc([System.DateTime]::UtcNow, $tz)
}

function Test-QuietHours([datetime]$now) {
    $s = [datetime]::ParseExact($QuietStart, "HH:mm", $null)
    $e = [datetime]::ParseExact($QuietEnd, "HH:mm", $null)
    $t = $now.TimeOfDay
    if ($s.TimeOfDay -lt $e.TimeOfDay) { return ($t -ge $s.TimeOfDay -and $t -lt $e.TimeOfDay) }
    return ($t -ge $s.TimeOfDay -or $t -lt $e.TimeOfDay)
}

$lastKibitz = (Get-Item $kibitz).LastWriteTimeUtc
$lastBack = (Get-Item $backtalk).LastWriteTimeUtc

Write-Host ("KIBITZ-poller watching. Interval {0}s, max {1} polls. Quiet {2}-{3} Vienna. Ctrl+C stops." -f $IntervalSec, $MaxPolls, $QuietStart, $QuietEnd)

for ($i = 1; $i -le $MaxPolls; $i++) {
    $now = Get-ViennaNow
    if (Test-QuietHours $now) {
        Write-Host ("Quiet hours ({0:HH:mm} Vienna). Exiting, resume in the morning." -f $now)
        break
    }
    Start-Sleep -Seconds $IntervalSec
    $k = (Get-Item $kibitz).LastWriteTimeUtc
    $b = (Get-Item $backtalk).LastWriteTimeUtc
    if ($k -gt $lastKibitz) {
        $lastKibitz = $k
        Write-Host ("`n[{0}] NEW KIBITZ.md entry. Relay to BUILDER chat:" -f $now.ToString("HH:mm"))
        Get-Content -Path $kibitz -Tail $TailLines | Write-Host
    }
    if ($b -gt $lastBack) {
        $lastBack = $b
        Write-Host ("`n[{0}] NEW KIBITZ_BACKTALK.md reply. Relay to KIBITZER chat:" -f $now.ToString("HH:mm"))
        Get-Content -Path $backtalk -Tail $TailLines | Write-Host
    }
    Write-Host ("poll {0}/{1} {2:HH:mm} Vienna. quiet." -f $i, $MaxPolls, (Get-ViennaNow))
}

Write-Host "Poller done. No tokens spent. Restart with -MaxPolls N for another shift."
