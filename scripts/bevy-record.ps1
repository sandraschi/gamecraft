# Record a passing Bevy gate where the catalog reads it: bevy/games/<slug>/verify.json.
# Usage: pwsh -File scripts/bevy-record.ps1 <slug> "<win line printed by the game>"
param([Parameter(Mandatory = $true)][string]$Slug, [Parameter(Mandatory = $true)][string]$Progress)
$dir = Join-Path $PSScriptRoot "..\bevy\games\$Slug"
if (-not (Test-Path $dir)) { Write-Host "no catalog entry for $Slug"; exit 1 }
$record = [ordered]@{
    pass      = $true
    failed    = $null
    progress  = $Progress.Trim()
    date      = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
    self_test = $null
}
$json = $record | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $dir 'verify.json'), $json, (New-Object Text.UTF8Encoding($false)))
Write-Host "recorded $Slug gate in bevy/games/$Slug/verify.json"