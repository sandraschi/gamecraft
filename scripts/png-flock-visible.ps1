# Fails when a screenshot shows too few flock-coloured pixels (warm white, tan, pale
# blue: bright and low-saturation). Counts every 2nd pixel in x and y.
param(
    [Parameter(Mandatory = $true)][string]$Path,
    [int]$Min = 1500
)
Add-Type -AssemblyName System.Drawing
$bmp = [System.Drawing.Bitmap]::FromFile((Resolve-Path $Path))
$hits = 0
for ($y = 0; $y -lt $bmp.Height; $y += 2) {
    for ($x = 0; $x -lt $bmp.Width; $x += 2) {
        $c = $bmp.GetPixel($x, $y)
        if ($c.R -gt 150 -and $c.G -gt 140 -and $c.B -gt 100) { $hits++ }
    }
}
$bmp.Dispose()
Write-Host "flock-coloured samples: $hits (need $Min): $Path"
if ($hits -lt $Min) { exit 1 }