param(
  [string]$PythonPath = "python",
  [switch]$SkipInstall,
  [switch]$UseCurrentPython
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$clientRoot = Split-Path -Parent $scriptRoot
$repoRoot = Split-Path -Parent $clientRoot
$tauriRoot = Join-Path $clientRoot "src-tauri"
$runnerScript = Join-Path $tauriRoot "rapidocr\rapidocr_runner.py"
$requirements = Join-Path $tauriRoot "rapidocr\requirements.txt"
$resourceDir = Join-Path $tauriRoot "resources\rapidocr"
$buildRoot = Join-Path $tauriRoot "target\rapidocr-runner-build"
$venvDir = Join-Path $buildRoot ".venv"
$workDir = Join-Path $buildRoot "work"
$specDir = Join-Path $buildRoot "spec"
$runnerDir = Join-Path $resourceDir "rapidocr-runner"
$runnerExe = Join-Path $runnerDir "rapidocr-runner.exe"
$staleOneFileRunnerExe = Join-Path $resourceDir "rapidocr-runner.exe"
$modelRoot = Join-Path $repoRoot "models\rapidocr"
$v6ModelRoot = Join-Path $repoRoot "ocrv6"

if (-not (Test-Path -LiteralPath $runnerScript)) {
  throw "RapidOCR runner script not found: $runnerScript"
}
if (-not (Test-Path -LiteralPath $requirements)) {
  throw "RapidOCR requirements file not found: $requirements"
}

New-Item -ItemType Directory -Path $resourceDir -Force | Out-Null
New-Item -ItemType Directory -Path $workDir -Force | Out-Null
New-Item -ItemType Directory -Path $specDir -Force | Out-Null
Remove-Item -LiteralPath $runnerDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $staleOneFileRunnerExe -Force -ErrorAction SilentlyContinue

$buildPython = $PythonPath
if (-not $UseCurrentPython) {
  $venvPython = Join-Path $venvDir "Scripts\python.exe"
  if (-not (Test-Path -LiteralPath $venvPython)) {
    & $PythonPath -m venv $venvDir
    if ($LASTEXITCODE -ne 0) {
      throw "Failed to create RapidOCR runner virtual environment."
    }
  }
  $buildPython = $venvPython
}

if (-not $SkipInstall) {
  & $buildPython -m pip install --upgrade pip
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to upgrade pip in the RapidOCR runner environment."
  }
  & $buildPython -m pip install -r $requirements
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to install RapidOCR runner requirements."
  }
}

& $buildPython -m PyInstaller `
  --clean `
  --noconfirm `
  --onedir `
  --name rapidocr-runner `
  --distpath $resourceDir `
  --workpath $workDir `
  --specpath $specDir `
  --collect-data rapidocr `
  --collect-binaries onnxruntime `
  --hidden-import _socket `
  --hidden-import select `
  --hidden-import _ssl `
  --hidden-import _hashlib `
  --hidden-import _bz2 `
  --hidden-import _lzma `
  --exclude-module torch `
  --exclude-module paddle `
  --exclude-module tensorflow `
  --exclude-module transformers `
  --exclude-module pandas `
  --exclude-module scipy `
  --exclude-module sklearn `
  --exclude-module matplotlib `
  --exclude-module yt_dlp `
  --exclude-module fastapi `
  --exclude-module av `
  $runnerScript

if ($LASTEXITCODE -ne 0) {
  throw "PyInstaller failed to build rapidocr-runner.exe."
}
if (-not (Test-Path -LiteralPath $runnerExe)) {
  throw "RapidOCR runner build did not create: $runnerExe"
}

$internalDir = Join-Path $runnerDir "_internal"
foreach ($requiredPattern in @("_socket*.pyd", "select*.pyd")) {
  $found = Get-ChildItem -Path $internalDir -Recurse -Filter $requiredPattern -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $found) {
    throw "RapidOCR runner is missing required Python extension: $requiredPattern"
  }
}

New-Item -ItemType Directory -Path $modelRoot -Force | Out-Null

& $runnerExe --warm-models --model-root $modelRoot
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed to warm RapidOCR model assets."
}

& $runnerExe --probe --model-version v5 --model-root $modelRoot
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed the V5 probe."
}

& $runnerExe --probe --model-version v4 --model-root $modelRoot
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed the V4 probe."
}

if (-not (Test-Path -LiteralPath $v6ModelRoot)) {
  throw "PP-OCRv6 model root not found: $v6ModelRoot"
}
& $runnerExe --probe --model-version v6 --model-root $v6ModelRoot
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed the V6 fixed-input contract probe."
}

$bundledModelDir = Join-Path $runnerDir "_internal\rapidocr\models"
if (Test-Path -LiteralPath $bundledModelDir) {
  Remove-Item -LiteralPath $bundledModelDir -Recurse -Force
}

Write-Host "RapidOCR runner built: $runnerExe" -ForegroundColor Green
Write-Host "RapidOCR models: $modelRoot" -ForegroundColor Green
