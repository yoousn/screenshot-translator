# 截图翻译 - 本地打包 & 发布脚本
# 用法:
#   .\pack_release.ps1                  # 仅打包 zip
#   .\pack_release.ps1 -Version v0.4.0  # 打包 + 打 tag + 推送 + 上传到 GitHub Release

param(
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$releaseDir  = Join-Path $projectRoot "release"
$zipName     = "ScreenshotTranslator_Windows.zip"
$zipPath     = Join-Path $projectRoot $zipName
$repo        = "yoousn/screenshot-translator"

# ── 1. 检查 release 目录 ──
if (-not (Test-Path (Join-Path $releaseDir "ScreenshotTranslator.exe"))) {
    Write-Host "[X] release\ScreenshotTranslator.exe does not exist, please build first!" -ForegroundColor Red
    Write-Host "   cmake -S client -B build -G Ninja -DCMAKE_BUILD_TYPE=Release"
    Write-Host "   cmake --build build --config Release"
    exit 1
}

# ── 2. 打包 zip ──
Write-Host "[*] Packing $zipName ..." -ForegroundColor Cyan
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path "$releaseDir\*" -DestinationPath $zipPath -Force
$sizeMB = [math]::Round((Get-Item $zipPath).Length / 1MB, 2)
Write-Host "[OK] Packed: $zipPath ($sizeMB MB)" -ForegroundColor Green

# ── 3. 如果指定了版本号，打 tag + 推送 + 上传 ──
if ($Version -ne "") {
    Write-Host ""
    Write-Host "[*] Releasing $Version ..." -ForegroundColor Cyan

    Push-Location $projectRoot

    # 提交所有更改（不包含 zip）
    git add -A
    git reset -- $zipName  # 确保 zip 不被提交到仓库
    git commit -m "release: $Version" --allow-empty

    # 打 tag 并推送
    git tag -a $Version -m "Release $Version"
    git push origin main
    git push origin $Version

    Pop-Location

    # 等几秒让 GitHub Actions 创建 draft release
    Write-Host "[*] Waiting for GitHub to create release page..." -ForegroundColor Yellow
    Start-Sleep -Seconds 10

    # 获取 GitHub token
    $cred = git credential fill @("protocol=https", "host=github.com", "")
    $token = ($cred | Where-Object { $_ -match "^password=" }) -replace "^password=", ""

    if (-not $token) {
        Write-Host "[!] Cannot get GitHub token. Please upload zip manually:" -ForegroundColor Yellow
        Write-Host "    https://github.com/$repo/releases/tag/$Version"
        exit 0
    }

    $headers = @{
        Authorization = "Bearer $token"
        Accept = "application/vnd.github.v3+json"
    }

    # 查找刚创建的 release
    $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases" -Headers $headers
    $release = $releases | Where-Object { $_.tag_name -eq $Version } | Select-Object -First 1

    if (-not $release) {
        # 如果 Actions 还没创建，手动创建
        Write-Host "[*] Creating release manually..." -ForegroundColor Yellow
        $body = @{
            tag_name = $Version
            name = "Screenshot Translator $Version"
            body = "## Screenshot Translator $Version`n`nDownload ``ScreenshotTranslator_Windows.zip``, extract and run."
            draft = $false
        } | ConvertTo-Json
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases" -Method Post -Headers $headers -Body $body -ContentType "application/json"
    }

    # 上传 zip 到 release
    $uploadUrl = $release.upload_url -replace '\{.*\}', ''
    $uploadUrl = "$uploadUrl`?name=$zipName"

    Write-Host "[*] Uploading $zipName to release..." -ForegroundColor Cyan
    $uploadHeaders = @{
        Authorization = "Bearer $token"
        "Content-Type" = "application/zip"
    }
    Invoke-RestMethod -Uri $uploadUrl -Method Post -Headers $uploadHeaders -InFile $zipPath | Out-Null

    # 如果是 draft，发布它
    if ($release.draft) {
        Invoke-RestMethod -Uri $release.url -Method Patch -Headers $headers -Body '{"draft":false}' -ContentType "application/json" | Out-Null
    }

    Write-Host "[OK] Release $Version published!" -ForegroundColor Green
    Write-Host "     https://github.com/$repo/releases/tag/$Version" -ForegroundColor Blue

    # 清理本地 zip
    Remove-Item $zipPath -Force

} else {
    Write-Host ""
    Write-Host "[i] To publish to GitHub, run:" -ForegroundColor Yellow
    Write-Host "   .\pack_release.ps1 -Version v0.4.0"
}
