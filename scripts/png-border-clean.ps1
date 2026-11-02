# Fails when any pixel on the outer 2 px frame of a screenshot differs from the
# background colour (top-left pixel): the scene is clipped by the window edge.
param([Parameter(Mandatory = $true)][string]$Path)
Add-Type -AssemblyName System.Drawing
$bmp = [System.Drawing.Bitmap]::FromFile((Resolve-Path $Path))
$bg = $bmp.GetPixel(0, 0)
$bad = 0
$w = $bmp.Width
$h = $bmp.Height
foreach ($x in 0..($w - 1)) {
    foreach ($y in 0, 1, ($h - 2), ($h - 1)) {
        if ($bmp.GetPixel($x, $y) -ne $bg) { $bad++ }
    }
}
foreach ($y in 0..($h - 1)) {
    foreach ($x in 0, 1, ($w - 2), ($w - 1)) {
        if ($bmp.GetPixel($x, $y) -ne $bg) { $bad++ }
    }
}
$bmp.Dispose()
if ($bad) {
    Write-Host "clipped at the screenshot border ($bad px): $Path"
    exit 1
}
Write-Host "border clean: $Path"