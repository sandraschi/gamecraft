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

Get-ChildItem depot/games | ForEach-Object {
    $out = & $godot --headless --path $_.FullName --fixed-fps 60 --quit-after $budget 2>&1 | Out-String
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
