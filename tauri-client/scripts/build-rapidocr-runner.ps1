param(
  [string]$PythonPath = "python",
  [switch]$SkipInstall,
  [switch]$UseCurrentPython
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$clientRoot = Split-Path -Parent $scriptRoot
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

& $runnerExe --warm-models
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed to warm RapidOCR model assets."
}

& $runnerExe --probe --model-version v5
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed the V5 probe."
}

& $runnerExe --probe --model-version v4
if ($LASTEXITCODE -ne 0) {
  throw "Built rapidocr-runner.exe failed the V4 probe."
}

Write-Host "RapidOCR runner built: $runnerExe" -ForegroundColor Green
