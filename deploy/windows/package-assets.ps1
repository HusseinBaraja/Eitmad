#Requires -Version 5.1
param([Parameter(Mandatory)][string]$Directory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
[void][IO.Directory]::CreateDirectory($Directory)
foreach ($asset in @(@('StoreLogo.png', 50), @('Square44x44Logo.png', 44), @('Square150x150Logo.png', 150))) {
    $size = [int]$asset[1]
    $bitmap = New-Object Drawing.Bitmap($size, $size)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $brush = New-Object Drawing.SolidBrush([Drawing.Color]::FromArgb(245, 238, 216))
    try {
        $graphics.Clear([Drawing.Color]::FromArgb(18, 62, 66))
        # Fixed native geometric furniture mark: seat, back, and legs.
        foreach ($rectangle in @(@(.23,.22,.16,.36), @(.23,.53,.54,.14), @(.23,.65,.12,.16), @(.65,.65,.12,.16))) {
            $graphics.FillRectangle($brush, [single]($size*$rectangle[0]), [single]($size*$rectangle[1]),
                [single]($size*$rectangle[2]), [single]($size*$rectangle[3]))
        }
        $bitmap.Save((Join-Path $Directory $asset[0]), [Drawing.Imaging.ImageFormat]::Png)
    } finally { $brush.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
}
