# Record or verify the committed example replays in docs/replays.
#
#   scripts\replays.ps1          re-record every example (after an intended rules change)
#   scripts\replays.ps1 -Check   verify every *.gcraft in docs\replays; fail if any no longer
#                                matches a fresh simulation, or if an example is missing
#
# Replays are seed + config + input log. Verifying one re-simulates it from scratch, so this
# gate fails the day someone changes maze or flock rules without meaning to.
param([switch]$Check)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$dir = Join-Path $root 'docs\replays'
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "gcraft-replays-$PID"
New-Item -ItemType Directory -Force -Path $dir, $tmp | Out-Null

Push-Location $root
try {
    cargo build -q -p gamecraft-cli
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    $cli = Join-Path $root 'target\debug\gamecraft-cli.exe'

    # PowerShell 5 strips double quotes from native arguments, so params go through @file.
    function Invoke-Cmd([string]$id, [hashtable]$params) {
        $file = Join-Path $tmp ("$id.json" -replace '[^\w\.\-]', '_')
        [System.IO.File]::WriteAllText($file, ($params | ConvertTo-Json -Compress))
        & $cli run --cmd "$id=@$file"
        if ($LASTEXITCODE -ne 0) { throw "$id failed (exit $LASTEXITCODE)" }
    }
    function Slash([string]$p) { $p -replace '\\', '/' }

    $examples = @(
        @{ name = 'maze-depot';        params = @{ game = 'maze'; level = 'depot' } },
        @{ name = 'maze-challenge';    params = @{ game = 'maze'; level = 'challenge' } },
        @{ name = 'murmuration-2000';  params = @{ game = 'murmuration'; count = 2000; seed = 3; ticks = 9000 } }
    )

    if ($Check) {
        foreach ($e in $examples) {
            if (-not (Test-Path (Join-Path $dir "$($e.name).gcraft"))) { throw "missing example docs\replays\$($e.name).gcraft; run scripts\replays.ps1" }
        }
        $files = Get-ChildItem $dir -Filter *.gcraft
        foreach ($f in $files) {
            Invoke-Cmd 'replay.verify' @{ path = (Slash $f.FullName) }
        }
        "replays-check: $($files.Count) replays verified"
    } else {
        foreach ($e in $examples) {
            $p = $e.params.Clone()
            $p.out = Slash (Join-Path $dir "$($e.name).gcraft")
            $p.overwrite = $true
            Invoke-Cmd 'replay.record' $p
        }
    }
} finally {
    Pop-Location
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
