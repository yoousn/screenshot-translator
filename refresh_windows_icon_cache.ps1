param(
    [switch]$IncludeThumbnailCache,
    [switch]$SkipExplorerRestart,
    [switch]$WhatIf
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param(
        [string]$Message
    )

    Write-Host "[icon-cache] $Message" -ForegroundColor Cyan
}

function Remove-CacheFiles {
    param(
        [string[]]$Paths
    )

    $removed = 0
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path)) {
            continue
        }

        if ($WhatIf) {
            Write-Host "[whatif] remove $path" -ForegroundColor Yellow
            $removed++
            continue
        }

        Remove-Item -LiteralPath $path -Force -ErrorAction Stop
        Write-Host "[removed] $path" -ForegroundColor Green
        $removed++
    }

    return $removed
}

$localAppData = [Environment]::GetFolderPath("LocalApplicationData")
$explorerCacheDir = Join-Path $localAppData "Microsoft\Windows\Explorer"

$cacheFiles = New-Object System.Collections.Generic.List[string]

$rootIconCache = Join-Path $localAppData "IconCache.db"
if (Test-Path -LiteralPath $rootIconCache) {
    $cacheFiles.Add($rootIconCache)
}

if (Test-Path -LiteralPath $explorerCacheDir) {
    Get-ChildItem -LiteralPath $explorerCacheDir -Filter "iconcache*" -File -ErrorAction SilentlyContinue |
        ForEach-Object { $cacheFiles.Add($_.FullName) }

    if ($IncludeThumbnailCache) {
        Get-ChildItem -LiteralPath $explorerCacheDir -Filter "thumbcache*" -File -ErrorAction SilentlyContinue |
            ForEach-Object { $cacheFiles.Add($_.FullName) }
    }
}

$cacheFiles = $cacheFiles |
    Sort-Object -Unique

if (-not $cacheFiles -or $cacheFiles.Count -eq 0) {
    Write-Step "No icon cache files were found."
    exit 0
}

Write-Step "Preparing to clean $($cacheFiles.Count) cache files."
if ($IncludeThumbnailCache) {
    Write-Step "Thumbnail cache is included."
}

if ($WhatIf) {
    Write-Step "WhatIf mode is enabled. Explorer will not be stopped and files will not be deleted."
} else {
    Write-Step "Stopping explorer.exe ..."
    Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 1200
}

$removedCount = Remove-CacheFiles -Paths $cacheFiles
Write-Step "Processed $removedCount cache files."

if (-not $WhatIf -and -not $SkipExplorerRestart) {
    Write-Step "Restarting explorer.exe ..."
    Start-Process explorer.exe
}

if (-not $WhatIf -and $SkipExplorerRestart) {
    Write-Step "Explorer restart was skipped. Start it manually when ready."
}

Write-Step "Done."
