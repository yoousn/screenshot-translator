param(
  [string]$RunnerPath,
  [string]$FixtureDir,
  [switch]$KeepFixtures
)

$ErrorActionPreference = "Stop"
$env:PYTHONIOENCODING = "utf-8"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$clientRoot = Split-Path -Parent $scriptRoot
$repoRoot = Split-Path -Parent $clientRoot
$tauriRoot = Join-Path $clientRoot "src-tauri"
$manifestPath = Join-Path $scriptRoot "ocr-v6-fixtures.json"
$pythonRunner = Join-Path $tauriRoot "rapidocr\rapidocr_runner.py"
$bundledRunner = Join-Path $tauriRoot "resources\rapidocr\rapidocr-runner\rapidocr-runner.exe"
$venvPython = Join-Path $tauriRoot "target\rapidocr-runner-build\.venv\Scripts\python.exe"
$pythonPath = if (Test-Path -LiteralPath $venvPython) { $venvPython } else { "python" }
$modelRoot = Join-Path $repoRoot "ocrv6"

if (-not $RunnerPath) {
  $RunnerPath = if (Test-Path -LiteralPath $bundledRunner) { $bundledRunner } else { $pythonRunner }
}
if (-not $FixtureDir) {
  $FixtureDir = Join-Path $env:TEMP "ppocr-v6-fixtures"
}
if (-not (Test-Path -LiteralPath $RunnerPath)) {
  throw "Local OCR runner is not ready: $RunnerPath"
}
if (-not (Test-Path -LiteralPath $modelRoot)) {
  throw "PP-OCRv6 model root is not ready: $modelRoot"
}
if (-not (Test-Path -LiteralPath $manifestPath)) {
  throw "PP-OCRv6 fixture manifest is missing: $manifestPath"
}

Add-Type -AssemblyName System.Drawing

function Invoke-V6Runner {
  param([Parameter(Mandatory = $true)][string[]]$Arguments)

  $outputLines = if ($RunnerPath.EndsWith(".py", [System.StringComparison]::OrdinalIgnoreCase)) {
    & $pythonPath $RunnerPath @Arguments
  } else {
    & $RunnerPath @Arguments
  }
  if ($LASTEXITCODE -ne 0) {
    throw "Local OCR runner failed with exit code $LASTEXITCODE"
  }
  $jsonLine = @($outputLines) | Where-Object { $_.TrimStart().StartsWith("{") } | Select-Object -Last 1
  if (-not $jsonLine) {
    throw "Local OCR runner did not return JSON"
  }
  return $jsonLine | ConvertFrom-Json
}

function New-OcrFixtureImage {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string[]]$Lines,
    [Parameter(Mandatory = $true)][string]$FontName,
    [Parameter(Mandatory = $true)][single]$FontSize,
    [Parameter(Mandatory = $true)][int]$Width,
    [Parameter(Mandatory = $true)][int]$Height
  )

  $bitmap = [System.Drawing.Bitmap]::new($Width, $Height)
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $font = $null
  $brush = $null
  try {
    $graphics.Clear([System.Drawing.Color]::White)
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    $font = [System.Drawing.Font]::new($FontName, $FontSize, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Point)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(20, 24, 31))
    $lineY = 28
    foreach ($line in $Lines) {
      $graphics.DrawString($line, $font, $brush, 36, $lineY)
      $lineY += [int]($FontSize * 1.85)
    }
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    if ($brush) { $brush.Dispose() }
    if ($font) { $font.Dispose() }
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

function Normalize-OcrText {
  param([string]$Text)
  return [System.Text.RegularExpressions.Regex]::Replace(($Text.Trim()), "\s+", " ")
}

function Get-LevenshteinDistance {
  param(
    [Parameter(Mandatory = $true)][string]$Expected,
    [Parameter(Mandatory = $true)][string]$Actual
  )

  $previous = New-Object int[] ($Actual.Length + 1)
  $current = New-Object int[] ($Actual.Length + 1)
  for ($j = 0; $j -le $Actual.Length; $j++) { $previous[$j] = $j }
  for ($i = 1; $i -le $Expected.Length; $i++) {
    $current[0] = $i
    for ($j = 1; $j -le $Actual.Length; $j++) {
      $cost = if ($Expected[$i - 1] -ceq $Actual[$j - 1]) { 0 } else { 1 }
      $current[$j] = [Math]::Min(
        [Math]::Min($current[$j - 1] + 1, $previous[$j] + 1),
        $previous[$j - 1] + $cost
      )
    }
    $swap = $previous
    $previous = $current
    $current = $swap
  }
  return $previous[$Actual.Length]
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$probe = Invoke-V6Runner -Arguments @("--probe", "--model-version", "v6", "--model-root", $modelRoot)
foreach ($property in @("dictionarySize", "classCount", "blankIndex", "spaceIndex", "spaceValue")) {
  if ($probe.contract.$property -cne $manifest.contract.$property) {
    throw "PP-OCRv6 contract mismatch for ${property}: expected '$($manifest.contract.$property)', got '$($probe.contract.$property)'"
  }
}

Remove-Item -LiteralPath $FixtureDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $FixtureDir -Force | Out-Null

try {
  foreach ($fixture in $manifest.fixtures) {
    $imagePath = Join-Path $FixtureDir "$($fixture.name).png"
    New-OcrFixtureImage `
      -Path $imagePath `
      -Lines @($fixture.lines) `
      -FontName $fixture.fontName `
      -FontSize ([single]$fixture.fontSize) `
      -Width ([int]$fixture.width) `
      -Height ([int]$fixture.height)

    $result = Invoke-V6Runner -Arguments @(
      "--image", $imagePath,
      "--model-version", "v6",
      "--mode", "auto",
      "--model-root", $modelRoot,
      "--no-small-text-retry"
    )
    if ($result.status -ne "success") {
      throw "PP-OCRv6 fixture '$($fixture.name)' failed: $($result.error)"
    }

    $actual = Normalize-OcrText -Text ((@($result.blocks) | ForEach-Object { $_.text }) -join " ")
    $expected = Normalize-OcrText -Text $fixture.expectedText
    $distance = Get-LevenshteinDistance -Expected $expected -Actual $actual
    $cer = if ($expected.Length -eq 0) { if ($actual.Length -eq 0) { 0.0 } else { 1.0 } } else { $distance / [double]$expected.Length }
    Write-Host "Fixture $($fixture.name): CER=$([Math]::Round($cer, 4)); text=$actual" -ForegroundColor Cyan
    if ($cer -gt [double]$fixture.maxCer) {
      throw "Fixture '$($fixture.name)' exceeded max CER $($fixture.maxCer): actual CER $cer; expected '$expected'; got '$actual'"
    }

    foreach ($token in @($fixture.criticalTokens)) {
      if ($actual.IndexOf($token, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
        throw "Fixture '$($fixture.name)' is missing critical token '$token': $actual"
      }
    }
    foreach ($token in @($fixture.caseSensitiveTokens)) {
      if ($actual.IndexOf($token, [System.StringComparison]::Ordinal) -lt 0) {
        throw "Fixture '$($fixture.name)' is missing case-sensitive token '$token': $actual"
      }
    }
  }
} finally {
  if (-not $KeepFixtures) {
    Remove-Item -LiteralPath $FixtureDir -Recurse -Force -ErrorAction SilentlyContinue
  } else {
    Write-Host "PP-OCRv6 fixtures kept at: $FixtureDir" -ForegroundColor Yellow
  }
}

Write-Host "PP-OCRv6 contract and fixture gates passed." -ForegroundColor Green
