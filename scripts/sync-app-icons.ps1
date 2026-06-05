# Sync the bundle + taskbar icons from a single source PNG (the original
# icon_fixed.png). This script does NOT redraw the icon - it just keeps
# the user's design intact and resizes it crisply for each output size.
#
# Size policy:
#   - 16x16 is intentionally skipped. Windows 10/11 taskbar is 32x32 by
#     default, and at 16 the original glow halo turns into a muddy blob.
#     Modern DPI scales (100/125/150/200%) all use 32+ pixels.
#   - icon.ico carries 32, 48, 64, 128, 256 (one for each common DPI step).
#   - taskbar.ico carries 32, 48, 64 (the actual sizes taskbars pick from).
#
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sync-app-icons.ps1

param(
    [string]$Source = "tauri-client\src-tauri\icons\icon.png",
    [string]$IconsDir = "tauri-client\src-tauri\icons"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Source)) {
    throw "Source icon not found: $Source. Expected the already synced original icon."
}
if (-not (Test-Path -LiteralPath $IconsDir)) {
    throw "Icons directory not found: $IconsDir"
}

Add-Type -AssemblyName System.Drawing

$sourceFull = (Resolve-Path -LiteralPath $Source).Path
$iconsFull  = (Resolve-Path -LiteralPath $IconsDir).Path

Write-Host "Source (original user icon): $sourceFull" -ForegroundColor Cyan
Write-Host "Target icons dir:             $iconsFull"  -ForegroundColor Cyan

# icon.png is the user's source - it must already be the original
# icon_fixed.png (1024x1024). This script never overwrites it; if you
# need to refresh it, copy 图标设计草案\icon_fixed.png over manually
# before running this script. We only write the derived sizes and
# the ICO files.
Write-Host "Source (kept as-is):   $sourceFull" -ForegroundColor Cyan
# Read the source into a byte buffer and decode from there, so the
# file handle on icon.png is released immediately. Otherwise Windows
# indexing / Defender can briefly lock the file and a later overwrite
# from this same script would fail with "file in use".
$sourceBytes = [System.IO.File]::ReadAllBytes($sourceFull)
$sourceStream = New-Object System.IO.MemoryStream(,$sourceBytes)
$master = [System.Drawing.Image]::FromStream($sourceStream)

function Save-AtSize([int]$size, [string]$destPath) {
    # Always render from the (much larger) master so we keep detail.
    # For <=48 we still use HighQualityBicubic, but we also run an
    # unsharp-style contrast bump on the alpha so the dark tile edge
    # doesn't melt into the taskbar background.
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode     = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode   = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($master, 0, 0, $size, $size)
    $g.Dispose()

    if ($size -le 64) {
        # Reinforce the tile edge: any pixel with mid-alpha gets pushed to
        # either fully opaque (dark tile) or fully transparent (background).
        # This is what the OS would do at small sizes if it had a vector
        # source - we approximate it here so the tile doesn't bleed.
        for ($y = 0; $y -lt $size; $y++) {
            for ($x = 0; $x -lt $size; $x++) {
                $px = $bmp.GetPixel($x, $y)
                $a = $px.A
                if ($a -lt 32) {
                    $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(0, 0, 0, 0))
                } elseif ($a -gt 224) {
                    # keep
                } else {
                    # bias toward opaque (the tile interior) since the
                    # tile is solid black, the only soft edge is the
                    # 1-2px rounded corner.
                    $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255, $px.R, $px.G, $px.B))
                }
            }
        }
    }

    $bmp.Save($destPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

# 1. icon.png is left untouched (it's our source). Skip the master
#    resave step entirely.

# 2. PNG targets (NO 16x16 anywhere - per user request).
$pngTargets = @{
    "32x32.png"          = 32
    "128x128.png"        = 128
    "128x128@2x.png"     = 256
    "taskbar-32x32.png"  = 32
    "taskbar-64x64.png"  = 64
}

# Remove the 16x16 leftovers from previous runs so the new policy sticks.
$stale = @("taskbar-16x16.png", "16x16.png")
foreach ($name in $stale) {
    $p = Join-Path $iconsFull $name
    if (Test-Path -LiteralPath $p) {
        Remove-Item -LiteralPath $p -Force
        Write-Host "Removed stale $name" -ForegroundColor DarkYellow
    }
}

foreach ($pair in $pngTargets.GetEnumerator()) {
    $file = $pair.Key
    $size = $pair.Value
    $dest = Join-Path $iconsFull $file
    Save-AtSize $size $dest
    Write-Host ("Wrote PNG  {0,-22} {1}x{1}" -f $file, $size) -ForegroundColor Green
}

# 3. Multi-size ICO files.
# icon.ico: 32, 48, 64, 128, 256 (modern DPI ladder, no 16)
# taskbar.ico: 32, 48, 64 (taskbar-actual sizes)
$icoMainSizes    = @(32, 48, 64, 128, 256)
$icoTaskbarSizes = @(32, 48, 64)

function Build-NativeIcoBytes([int]$size) {
    # Use .NET's System.Drawing.Icon.Save to produce a Windows-native
    # 32bpp .ico payload. This is the format RC.EXE actually accepts -
    # manually constructed BITMAPV5HEADER ICOs trigger RC2176 "old DIB".
    # We render the source at $size, then dump the .NET icon's bytes.
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode     = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode   = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($master, 0, 0, $size, $size)
    $g.Dispose()

    # Soft-edge -> hard-edge cleanup for small sizes.
    if ($size -le 64) {
        for ($y = 0; $y -lt $size; $y++) {
            for ($x = 0; $x -lt $size; $x++) {
                $px = $bmp.GetPixel($x, $y)
                $a = $px.A
                if ($a -lt 32) {
                    $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(0, 0, 0, 0))
                } elseif ($a -lt 224) {
                    $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255, $px.R, $px.G, $px.B))
                }
            }
        }
    }

    # Icon.FromHandle premultiplies alpha internally (RC.EXE-acceptable),
    # so the bytes we read back are the same ones Windows expects.
    $hIcon = $bmp.GetHicon()
    try {
        $icon = [System.Drawing.Icon]::FromHandle($hIcon)
        $ms = New-Object System.IO.MemoryStream
        $icon.Save($ms)
        $bytes = $ms.ToArray()
        $ms.Dispose()
    } finally {
        # DestroyIcon, not FreeHGlobal (HICON != HGLOBAL)
        Add-Type -Namespace Ysn -Name IconUtil -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true)]
public static extern bool DestroyIcon(System.IntPtr hIcon);
"@
        [Ysn.IconUtil]::DestroyIcon($hIcon) | Out-Null
    }
    $bmp.Dispose()
    return $bytes
}

function Write-Ico($sizes, $destPath) {
    # Strategy: render each size as its own .ico via .NET, then strip
    # the ICONDIR header (6 bytes) from each and reassemble them under
    # a single new ICONDIR with one entry per size. This keeps the
    # ICONDIRENTRY layout correct while every embedded image is a
    # Windows-native 32bpp icon payload.
    $payloads = @()
    foreach ($s in $sizes) {
        $rawIco = Build-NativeIcoBytes $s
        # .NET ICO layout: 6-byte ICONDIR + 16-byte ICONDIRENTRY + image
        $imageOffset = 6 + 16
        $imageLength = $rawIco.Length - $imageOffset
        $imageBytes = New-Object 'System.Collections.Generic.List[byte]' $imageLength
        for ($i = 0; $i -lt $imageLength; $i++) { $imageBytes.Add($rawIco[$imageOffset + $i]) }
        $payloads += @{ Size = $s; Bytes = $imageBytes.ToArray() }
    }

    $out = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter($out)
    $bw.Write([UInt16]0)
    $bw.Write([UInt16]1)
    $bw.Write([UInt16]$payloads.Count)
    $dirBytes = 6 + 16 * $payloads.Count
    $offset = $dirBytes
    foreach ($p in $payloads) {
        $s = $p.Size
        $w = if ($s -ge 256) { 0 } else { $s }
        $h = if ($s -ge 256) { 0 } else { $s }
        $bw.Write([Byte]$w)
        $bw.Write([Byte]$h)
        $bw.Write([Byte]0)
        $bw.Write([Byte]0)
        $bw.Write([UInt16]1)
        $bw.Write([UInt16]32)
        $bw.Write([UInt32]$p.Bytes.Length)
        $bw.Write([UInt32]$offset)
        $offset += $p.Bytes.Length
    }
    foreach ($p in $payloads) {
        $bw.Write($p.Bytes)
    }
    $bw.Flush()
    [System.IO.File]::WriteAllBytes($destPath, $out.ToArray())
    $out.Dispose()
}

Write-Ico $icoMainSizes (Join-Path $iconsFull "icon.ico")
Write-Host ("Wrote ICO   icon.ico ({0})" -f ($icoMainSizes -join ",")) -ForegroundColor Green

Write-Ico $icoTaskbarSizes (Join-Path $iconsFull "taskbar.ico")
Write-Host ("Wrote ICO   taskbar.ico ({0})" -f ($icoTaskbarSizes -join ",")) -ForegroundColor Green

$master.Dispose()
Write-Host ""
Write-Host "Done. Only 32+ sizes generated - the original 16x16 muddy blur is gone." -ForegroundColor Cyan
