#Requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter()]
    [string] $SourceImage,

    [Parameter()]
    [string] $OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $SourceImage) {
    $SourceImage = Join-Path $repoRoot 'assets\icon-xl.png'
}
elseif (-not [System.IO.Path]::IsPathRooted($SourceImage)) {
    $SourceImage = Join-Path $repoRoot $SourceImage
}

if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot 'artifacts\store\generated-assets'
}
elseif (-not [System.IO.Path]::IsPathRooted($OutputDirectory)) {
    $OutputDirectory = Join-Path $repoRoot $OutputDirectory
}

$SourceImage = [System.IO.Path]::GetFullPath($SourceImage)
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

if (-not (Test-Path -LiteralPath $SourceImage -PathType Leaf)) {
    throw "Store source image was not found: $SourceImage"
}

Add-Type -AssemblyName System.Drawing

function Write-PngAsset {
    param(
        [Parameter(Mandatory)] [System.Drawing.Image] $Source,
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] [int] $CanvasWidth,
        [Parameter(Mandatory)] [int] $CanvasHeight,
        [Parameter(Mandatory)] [int] $ContentWidth,
        [Parameter(Mandatory)] [int] $ContentHeight
    )

    $bitmap = [System.Drawing.Bitmap]::new(
        $CanvasWidth,
        $CanvasHeight,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    try {
        $bitmap.SetResolution(96, 96)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.Clear([System.Drawing.Color]::Transparent)
            $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver
            $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality

            $x = [int] (($CanvasWidth - $ContentWidth) / 2)
            $y = [int] (($CanvasHeight - $ContentHeight) / 2)
            $destination = [System.Drawing.Rectangle]::new($x, $y, $ContentWidth, $ContentHeight)
            $graphics.DrawImage($Source, $destination)
        }
        finally {
            $graphics.Dispose()
        }

        $directory = Split-Path -Parent $Path
        New-Item -ItemType Directory -Force -Path $directory | Out-Null
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $bitmap.Dispose()
    }
}

if (Test-Path -LiteralPath $OutputDirectory) {
    [System.IO.Directory]::Delete($OutputDirectory, $true)
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

$source = [System.Drawing.Image]::FromFile($SourceImage)
try {
    $definitions = @(
        @{ Name = 'StoreLogo.png'; Width = 50; Height = 50; ContentWidth = 50; ContentHeight = 50 },
        @{ Name = 'Square44x44Logo.png'; Width = 44; Height = 44; ContentWidth = 44; ContentHeight = 44 },
        @{ Name = 'Square150x150Logo.png'; Width = 150; Height = 150; ContentWidth = 150; ContentHeight = 150 },
        @{ Name = 'Wide310x150Logo.png'; Width = 310; Height = 150; ContentWidth = 116; ContentHeight = 116 },
        @{ Name = 'SplashScreen.png'; Width = 620; Height = 300; ContentWidth = 192; ContentHeight = 192 }
    )

    foreach ($definition in $definitions) {
        $assetPath = Join-Path $OutputDirectory $definition.Name
        Write-PngAsset `
            -Source $source `
            -Path $assetPath `
            -CanvasWidth $definition.Width `
            -CanvasHeight $definition.Height `
            -ContentWidth $definition.ContentWidth `
            -ContentHeight $definition.ContentHeight

        $asset = [System.Drawing.Image]::FromFile($assetPath)
        try {
            if ($asset.Width -ne $definition.Width -or $asset.Height -ne $definition.Height) {
                throw "Generated asset has incorrect dimensions: $assetPath ($($asset.Width)x$($asset.Height))"
            }
        }
        finally {
            $asset.Dispose()
        }
    }
}
finally {
    $source.Dispose()
}

Write-Host 'Panopticon Store assets generated.' -ForegroundColor Green
Get-ChildItem -LiteralPath $OutputDirectory -File |
    Sort-Object Name |
    Select-Object Name, Length |
    Format-Table -AutoSize
