# Render the new taskbar icon on simulated light/dark taskbars
# (and several macOS Dock themes) so we can eyeball contrast before
# shipping a release build. Output: scripts/icon-contrast-check.png

param(
    [string]$IconPath = "tauri-client\src-tauri\icons\taskbar-32x32.png",
    [string]$OutPath  = "scripts\icon-contrast-check.png"
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

if (-not (Test-Path -LiteralPath $IconPath)) {
    throw "Icon not found: $IconPath"
}
$iconFull = (Resolve-Path -LiteralPath $IconPath).Path
$icon = [System.Drawing.Image]::FromFile($iconFull)

# 6 columns: Win11 light, Win11 dark, Win10 light, Win10 dark,
#            macOS light dock, macOS dark dock
$scenes = @(
    @{ Name = "Windows 11 (light taskbar)"; Bg = [System.Drawing.Color]::FromArgb(255, 243, 243, 243); Fg = [System.Drawing.Color]::FromArgb(255, 32, 32, 32) },
    @{ Name = "Windows 11 (dark taskbar)";  Bg = [System.Drawing.Color]::FromArgb(255, 32, 32, 32);  Fg = [System.Drawing.Color]::FromArgb(255, 240, 240, 240) },
    @{ Name = "Windows 10 (light taskbar)"; Bg = [System.Drawing.Color]::FromArgb(255, 230, 230, 230); Fg = [System.Drawing.Color]::FromArgb(255, 24, 24, 24) },
    @{ Name = "Windows 10 (dark taskbar)";  Bg = [System.Drawing.Color]::FromArgb(255, 28, 28, 28);  Fg = [System.Drawing.Color]::FromArgb(255, 220, 220, 220) },
    @{ Name = "macOS Dock (light)";         Bg = [System.Drawing.Color]::FromArgb(180, 220, 220, 220); Fg = [System.Drawing.Color]::FromArgb(255, 30, 30, 30) },
    @{ Name = "macOS Dock (dark)";          Bg = [System.Drawing.Color]::FromArgb(220, 30, 30, 30); Fg = [System.Drawing.Color]::FromArgb(255, 230, 230, 230) }
)

# Two rows: 32x32 and 64x64. We intentionally do NOT test 16x16 - the
# original icon is built for 32+ and at 16 the glow halo turns muddy.
$sizes = @(32, 64)

# Canvas: 6 columns, 2 rows
$colW = 240
$rowH = 110
$pad  = 24
$titleH = 36
$w = $colW * $scenes.Count + $pad * ($scenes.Count + 1)
$h = $titleH + ($rowH + $pad) * $sizes.Count + $pad

$canvas = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($canvas)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
$g.Clear([System.Drawing.Color]::FromArgb(255, 250, 250, 252))

# Title
$titleFont = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Bold)
$titleBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 50, 50, 50))
$g.DrawString("YSN icon contrast check (32x32 + 64x64 across light/dark taskbars & dock)",
    $titleFont, $titleBrush, [float]($pad), [float]($pad - 4))
$titleFont.Dispose()
$titleBrush.Dispose()

$labelFont = New-Object System.Drawing.Font("Segoe UI", 8.5)
$labelBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 70, 70, 70))
$bgBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::Black)
$iconRectBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(60, 0, 0, 0))

for ($c = 0; $c -lt $scenes.Count; $c++) {
    $scene = $scenes[$c]
    $colX = $pad + $c * ($colW + $pad)
    # column label
    $g.DrawString($scene.Name, $labelFont, $labelBrush, [float]$colX, [float]($titleH))
}

for ($r = 0; $r -lt $sizes.Count; $r++) {
    $size = $sizes[$r]
    $y = $titleH + $pad + $r * ($rowH + $pad)

    for ($c = 0; $c -lt $scenes.Count; $c++) {
        $scene = $scenes[$c]
        $colX = $pad + $c * ($colW + $pad)

        # taskbar background
        $bgRect = New-Object System.Drawing.RectangleF([float]$colX, [float]$y, [float]$colW, [float]$rowH)
        $bgBrush.Color = $scene.Bg
        $g.FillRectangle($bgBrush, $bgRect)

        # subtle 1px border
        $borderPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(40, 0, 0, 0), 1)
        $g.DrawRectangle($borderPen, [int]$colX, [int]$y, [int]$colW - 1, [int]$rowH - 1)
        $borderPen.Dispose()

        # icon centered, surrounded by a soft "platform" rectangle to simulate
        # Windows 11 taskbar / macOS Dock reflective backdrop
        $iconSize = $size
        $iconX = [int]($colX + ($colW - $iconSize) / 2)
        $iconY = [int]($y + ($rowH - $iconSize) / 2 - 6)
        $g.DrawImage($icon, $iconX, $iconY, $iconSize, $iconSize)

        # caption underneath
        $caption = "${size}x${size}"
        $captionSize = $g.MeasureString($caption, $labelFont)
        $captionBrush = New-Object System.Drawing.SolidBrush($scene.Fg)
        $g.DrawString($caption, $labelFont, $captionBrush,
            [float]($colX + ($colW - $captionSize.Width) / 2),
            [float]($y + $rowH - $captionSize.Height - 4))
        $captionBrush.Dispose()
    }
}

$labelFont.Dispose()
$labelBrush.Dispose()
$bgBrush.Dispose()
$iconRectBrush.Dispose()
$g.Dispose()
$icon.Dispose()

$dir = Split-Path -Parent $OutPath
if (-not (Test-Path -LiteralPath $dir)) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
}
$canvas.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$canvas.Dispose()
Write-Host "Wrote contrast check: $OutPath" -ForegroundColor Green
