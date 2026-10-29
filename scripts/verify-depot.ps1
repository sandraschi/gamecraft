# Depot win gate.
#
# Booting is not winning. `just depot-godot` proves a scene loads with no engine
# errors; this proves each game can actually be completed. Nine of the fifteen
# games boot clean and have never printed the win marker, so this gate is RED
# today by design: it is reporting the real state, not a new regression.
#
# Exits 1 if any game fails to print "goal reached" within its frame budget.

$godot = (Get-Command godot).Source
$budget = 3600
$fail = 0

Get-ChildItem depot/games | ForEach-Object {
    $out = & $godot --headless --path $_.FullName --fixed-fps 60 --quit-after $budget 2>&1 | Out-String
    if ($out -match 'goal reached') {
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
