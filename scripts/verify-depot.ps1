# Depot win gate.
#
# Booting is not winning. `just depot-godot` proves a scene loads with no engine
# errors; this proves each game can actually be completed.
#
# Win markers are per-archetype, because each runtime handler prints its own:
#   goal areas      -> "GameCraft: goal reached"
#   grid puzzles    -> "GameCraft: puzzle solved"
#   doors           -> "GameCraft: door opened, win"
#   combat          -> "GameCraft: all hostiles down, win"
# Matching only "goal reached" reported winnable games as unproven.
#
# Exits 1 if any game produces no win marker within its frame budget.

# Games no longer autoplay by default; the verifier drives them.
$env:GAMECRAFT_AUTOPLAY = '1'
$godot = (Get-Command godot).Source
$budget = 3600
$fail = 0
$pattern = 'goal reached|puzzle solved|door opened, win|all hostiles down'

# Wall-clock limit per game. A hung engine is killed instead of blocking the gate and
# being left behind (2026-10-31: leftover engines piled up on the dev box).
$timeoutSec = [int]($budget / 60 * 2 + 60)
$logDir = Join-Path $env:TEMP 'gamecraft-verify-depot'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

Get-ChildItem depot/games | ForEach-Object {
    $outFile = Join-Path $logDir ("{0}.out.txt" -f $_.Name)
    $errFile = Join-Path $logDir ("{0}.err.txt" -f $_.Name)
    $argList = @('--headless', '--path', ('"{0}"' -f $_.FullName), '--fixed-fps', '60', '--quit-after', "$budget")
    $p = Start-Process -FilePath $godot -ArgumentList $argList -NoNewWindow -PassThru `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    if (-not $p.WaitForExit($timeoutSec * 1000)) {
        # Kill the whole tree, not just the top process.
        & taskkill.exe /F /T /PID $p.Id | Out-Null
        $fail++
        Write-Host ("{0}: TIMEOUT after {1}s (killed)" -f $_.Name, $timeoutSec)
        return
    }
    $out = (Get-Content $outFile -Raw -ErrorAction SilentlyContinue) + (Get-Content $errFile -Raw -ErrorAction SilentlyContinue)
    if ($out -match $pattern) {
        Write-Host ("{0}: WIN" -f $_.Name)
    }
    else {
        $fail++
        Write-Host ("{0}: NO WIN MARKER" -f $_.Name)
    }
}

if ($fail -gt 0) {
    Write-Host ("{0} games unproven - they boot but have never shown a win" -f $fail)
    exit 1
}

Write-Host 'all games win'
