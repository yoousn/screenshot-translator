param(
    [string]$Version = "",
    [switch]$Build,
    [string]$OutputName = "ScreenshotTranslator_Windows.zip"
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $scriptDir "..\..")).Path
$portableDir = Join-Path $projectRoot "release\YSN-Screenshot-Translator"
$tauriConfig = Join-Path $projectRoot "tauri-client\src-tauri\tauri.conf.json"

$resolvedVersion = $Version.Trim()
if (-not $resolvedVersion) {
    if (Test-Path -LiteralPath $tauriConfig) {
        $resolvedVersion = ((Get-Content -LiteralPath $tauriConfig -Raw) | ConvertFrom-Json).version
    }
}
if (-not $resolvedVersion) {
    $resolvedVersion = "dev"
}
$safeVersion = $resolvedVersion -replace '[^\w\.-]', '_'
$artifactDir = Join-Path $projectRoot "build\x64_v$safeVersion"

if ($Build) {
    $buildScript = Join-Path $projectRoot "scripts\build\build.bat"
    & cmd /c "`"$buildScript`" --no-pause --no-launch"
    if ($LASTEXITCODE -ne 0) {
        throw "scripts\build\build.bat failed with exit code $LASTEXITCODE"
    }
}

if ($Version.Trim()) {
    $OutputName = "ScreenshotTranslator_Windows_$safeVersion.zip"
}

$required = @(
    (Join-Path $portableDir "YsnTrans.exe"),
    (Join-Path $portableDir "resources\rapidocr\rapidocr-runner\rapidocr-runner.exe"),
    (Join-Path $portableDir "models\rapidocr\ch_PP-OCRv5_det_mobile.onnx")
)

foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Portable package is incomplete. Missing: $path. Run .\scripts\build\build.bat first."
    }
}

New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null

$zipPath = Join-Path $artifactDir $OutputName
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}

Compress-Archive -Path (Join-Path $portableDir "*") -DestinationPath $zipPath -Force
$sizeMB = [math]::Round((Get-Item -LiteralPath $zipPath).Length / 1MB, 2)

Write-Host "Packed: $zipPath ($sizeMB MB)" -ForegroundColor Green
Write-Host "Upload or copy this zip manually. This script does not commit, tag, push, or publish." -ForegroundColor Yellow
