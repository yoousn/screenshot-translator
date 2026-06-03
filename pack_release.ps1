param(
    [string]$Version = "",
    [switch]$Build,
    [string]$OutputName = "ScreenshotTranslator_Windows.zip"
)

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$portableDir = Join-Path $projectRoot "release\YSN-Screenshot-Translator"

if ($Build) {
    $buildScript = Join-Path $projectRoot "build.bat"
    & cmd /c "`"$buildScript`" --no-pause"
    if ($LASTEXITCODE -ne 0) {
        throw "build.bat failed with exit code $LASTEXITCODE"
    }
}

if ($Version.Trim()) {
    $safeVersion = $Version.Trim() -replace '[^\w\.-]', '_'
    $OutputName = "ScreenshotTranslator_Windows_$safeVersion.zip"
}

$required = @(
    (Join-Path $portableDir "tauri-client.exe"),
    (Join-Path $portableDir "resources\rapidocr\rapidocr-runner\rapidocr-runner.exe"),
    (Join-Path $portableDir "models\rapidocr\ch_PP-OCRv5_det_mobile.onnx")
)

foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Portable package is incomplete. Missing: $path. Run .\build.bat first."
    }
}

$zipPath = Join-Path (Join-Path $projectRoot "release") $OutputName
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}

Compress-Archive -Path (Join-Path $portableDir "*") -DestinationPath $zipPath -Force
$sizeMB = [math]::Round((Get-Item -LiteralPath $zipPath).Length / 1MB, 2)

Write-Host "Packed: $zipPath ($sizeMB MB)" -ForegroundColor Green
Write-Host "Upload or copy this zip manually. This script does not commit, tag, push, or publish." -ForegroundColor Yellow
