param(
    [switch]$IncludeThumbnailCache,
    [switch]$SkipExplorerRestart,
    [switch]$WhatIf
)

$ErrorActionPreference = "Stop"
$failedPaths = [System.Collections.Generic.List[string]]::new()
$scheduledPaths = [System.Collections.Generic.List[string]]::new()
$explorerStopped = $false

function Write-Step {
    param([string]$Message)
    Write-Host "[icon-cache] $Message" -ForegroundColor Cyan
}

function Get-CacheFiles {
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    $explorerCacheDir = Join-Path $localAppData "Microsoft\Windows\Explorer"
    $paths = [System.Collections.Generic.List[string]]::new()
    $rootIconCache = Join-Path $localAppData "IconCache.db"

    if (Test-Path -LiteralPath $rootIconCache) {
        $paths.Add($rootIconCache)
    }
    if (Test-Path -LiteralPath $explorerCacheDir) {
        Get-ChildItem -LiteralPath $explorerCacheDir -Filter "iconcache*" -File -ErrorAction SilentlyContinue |
            ForEach-Object { $paths.Add($_.FullName) }
        if ($IncludeThumbnailCache) {
            Get-ChildItem -LiteralPath $explorerCacheDir -Filter "thumbcache*" -File -ErrorAction SilentlyContinue |
                ForEach-Object { $paths.Add($_.FullName) }
        }
    }

    return @($paths | Sort-Object -Unique)
}

if (-not ("Ysn.PendingDelete" -as [type])) {
    Add-Type -Namespace Ysn -Name PendingDelete -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode, SetLastError = true)]
public static extern bool MoveFileEx(string existingFileName, string newFileName, int flags);
"@
}

function Stop-ExplorerForCleanup {
    Get-Process -Name explorer -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    $script:explorerStopped = $true
    Start-Sleep -Milliseconds 300
}

function Remove-CacheBatch {
    param([string[]]$Paths)

    $remaining = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $Paths) {
        try {
            if (Test-Path -LiteralPath $path) {
                [System.IO.File]::SetAttributes($path, [System.IO.FileAttributes]::Normal)
                Remove-Item -LiteralPath $path -Force -ErrorAction Stop
                Write-Host "[removed] $path" -ForegroundColor Green
            }
        } catch {
            $remaining.Add($path)
        }
    }
    return @($remaining)
}

function Schedule-CacheDelete {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    if ([Ysn.PendingDelete]::MoveFileEx($Path, $null, 4)) {
        Write-Host "[scheduled] delete on next Windows restart: $Path" -ForegroundColor DarkYellow
        $script:scheduledPaths.Add($Path)
    } else {
        $errorCode = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
        if ($errorCode -eq 2 -or $errorCode -eq 3 -or -not (Test-Path -LiteralPath $Path)) {
            Write-Host "[cleared] cache file disappeared during cleanup: $Path" -ForegroundColor Green
            return
        }
        Write-Warning "Could not remove or schedule cache file: $Path (Win32 error $errorCode)"
        $script:failedPaths.Add($Path)
    }
}

function Remove-CacheFilesReliably {
    param([string[]]$Paths)

    $remaining = @($Paths)
    for ($attempt = 1; $attempt -le 5 -and $remaining.Count -gt 0; $attempt++) {
        Write-Step "Cleanup pass $attempt for $($remaining.Count) file(s) ..."
        Stop-ExplorerForCleanup
        $remaining = @(Remove-CacheBatch -Paths $remaining)
        if ($remaining.Count -gt 0) {
            Start-Sleep -Milliseconds 250
        }
    }
    foreach ($path in $remaining) {
        Schedule-CacheDelete -Path $path
    }
}

try {
    $initialFiles = Get-CacheFiles
    Write-Step "Found $($initialFiles.Count) icon/thumbnail cache file(s)."

    if ($WhatIf) {
        foreach ($path in $initialFiles) {
            Write-Host "[whatif] remove $path" -ForegroundColor Yellow
        }
        Write-Step "WhatIf complete. Explorer was not stopped."
        return
    }

    Remove-CacheFilesReliably -Paths $initialFiles

    $ie4uinit = Join-Path $env:SystemRoot "System32\ie4uinit.exe"
    if (Test-Path -LiteralPath $ie4uinit) {
        Write-Step "Requesting Windows icon cache refresh ..."
        try {
            $refresh = Start-Process -FilePath $ie4uinit -ArgumentList "-ClearIconCache" -PassThru -WindowStyle Hidden
            if (-not $refresh.WaitForExit(3000)) {
                Write-Warning "Windows icon refresh helper exceeded 3 seconds and was stopped."
                $refresh.Kill()
            }
        } catch {
            Write-Warning "Windows icon refresh helper could not run: $($_.Exception.Message)"
        }
    }
} finally {
    if (-not $WhatIf -and -not $SkipExplorerRestart) {
        Write-Step "Ensuring explorer.exe is running ..."
        if (-not (Get-Process -Name explorer -ErrorAction SilentlyContinue)) {
            Start-Process explorer.exe
            Start-Sleep -Milliseconds 800
        }
    } elseif (-not $WhatIf -and $SkipExplorerRestart -and $explorerStopped) {
        Write-Step "Explorer restart was explicitly skipped."
    }
}

if ($failedPaths.Count -gt 0) {
    Write-Warning "Cache cleanup completed with $($failedPaths.Count) unscheduled failure(s)."
    exit 1
}

if ($scheduledPaths.Count -gt 0) {
    Write-Step "$($scheduledPaths.Count) locked cache file(s) are scheduled for deletion on the next Windows restart."
}

Write-Step "Done."
