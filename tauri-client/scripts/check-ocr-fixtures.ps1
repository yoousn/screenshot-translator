param(
  [string]$RunnerPath,
  [string]$FixtureDir,
  [string]$RealScreenshotPath,
  [string]$UserScreenshotDir,
  [string[]]$RealExpectContains = @(),
  [int]$RealMinBlocks = 8,
  [switch]$SkipUserScreenshots,
  [switch]$KeepFixtures
)

$ErrorActionPreference = "Stop"

$env:PYTHONIOENCODING = "utf-8"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$clientRoot = Split-Path -Parent $scriptRoot
$repoRoot = Split-Path -Parent $clientRoot
$tauriRoot = Join-Path $clientRoot "src-tauri"
$bundledRunner = Join-Path $tauriRoot "resources\rapidocr\rapidocr-runner\rapidocr-runner.exe"
$pythonRunner = Join-Path $tauriRoot "rapidocr\rapidocr_runner.py"
$defaultRunner = if (Test-Path -LiteralPath $bundledRunner) { $bundledRunner } else { $pythonRunner }
$modelRoot = Join-Path $repoRoot "models\rapidocr"

if (-not $RunnerPath) {
  $RunnerPath = $defaultRunner
}
if (-not $FixtureDir) {
  $FixtureDir = Join-Path $env:TEMP "rapidocr-fixtures"
}
if (-not $UserScreenshotDir) {
  $UserScreenshotDir = Join-Path $repoRoot (-join (0x6D4B, 0x8BD5, 0x56FE, 0x7247 | ForEach-Object { [char]$_ }))
}
if (-not (Test-Path -LiteralPath $RunnerPath)) {
  throw "RapidOCR runner is not ready: $RunnerPath"
}
if (-not (Test-Path -LiteralPath $modelRoot)) {
  throw "RapidOCR model root is not ready: $modelRoot"
}

Add-Type -AssemblyName System.Drawing

function New-OcrFixtureImage {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string[]]$Lines,
    [Parameter(Mandatory = $true)][string]$FontName,
    [Parameter(Mandatory = $true)][single]$FontSize,
    [int]$Width = 900,
    [int]$Height = 220
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

function New-UnicodeString {
  param([Parameter(Mandatory = $true)][int[]]$CodePoints)
  return -join ($CodePoints | ForEach-Object { [char]$_ })
}

function Normalize-OcrText {
  param([string]$Text)
  return (($Text.ToLowerInvariant().ToCharArray() | Where-Object { -not [char]::IsWhiteSpace($_) }) -join "")
}

function Invoke-RapidOcrFixture {
  param([Parameter(Mandatory = $true)][string]$ImagePath)

  $outputLines = if ($RunnerPath.EndsWith(".py", [System.StringComparison]::OrdinalIgnoreCase)) {
    & python $RunnerPath --image $ImagePath --model-version v5 --mode auto --model-root $modelRoot
  } else {
    & $RunnerPath --image $ImagePath --model-version v5 --mode auto --model-root $modelRoot
  }
  if ($LASTEXITCODE -ne 0) {
    throw "RapidOCR runner failed for $ImagePath with exit code $LASTEXITCODE"
  }
  $jsonLine = @($outputLines) | Where-Object { $_.TrimStart().StartsWith("{") } | Select-Object -Last 1
  if (-not $jsonLine) {
    throw "RapidOCR runner did not return JSON for $ImagePath"
  }
  return $jsonLine | ConvertFrom-Json
}

Remove-Item -LiteralPath $FixtureDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $FixtureDir | Out-Null

$zhSavePreview = New-UnicodeString -CodePoints @(0x4FDD, 0x5B58, 0x524D, 0x6253, 0x5F00, 0x9884, 0x89C8)
$zhCopyTranslated = New-UnicodeString -CodePoints @(0x590D, 0x5236, 0x7FFB, 0x8BD1, 0x6587, 0x672C)
$koSavePreview = New-UnicodeString -CodePoints @(0xC800, 0xC7A5, 0xD558, 0xAE30, 0x0020, 0xC804, 0xC5D0, 0x0020, 0xBBF8, 0xB9AC, 0xBCF4, 0xAE30, 0x0020, 0xC5F4, 0xAE30)
$koCopyTranslated = New-UnicodeString -CodePoints @(0xBC88, 0xC5ED, 0xB41C, 0x0020, 0xD14D, 0xC2A4, 0xD2B8, 0x0020, 0xBCF5, 0xC0AC)
$koSave = New-UnicodeString -CodePoints @(0xC800, 0xC7A5, 0xD558, 0xAE30)
$koPreview = New-UnicodeString -CodePoints @(0xBBF8, 0xB9AC, 0xBCF4, 0xAE30)
$koCopy = New-UnicodeString -CodePoints @(0xBCF5, 0xC0AC)
$jaSavePreview = New-UnicodeString -CodePoints @(0x4FDD, 0x5B58, 0x3059, 0x308B, 0x524D, 0x306B, 0x30D7, 0x30EC, 0x30D3, 0x30E5, 0x30FC, 0x3092, 0x958B, 0x304F)
$jaCopyTranslated = New-UnicodeString -CodePoints @(0x7FFB, 0x8A33, 0x30C6, 0x30AD, 0x30B9, 0x30C8, 0x3092, 0x30B3, 0x30D4, 0x30FC)
$jaSave = New-UnicodeString -CodePoints @(0x4FDD, 0x5B58, 0x3059, 0x308B, 0x524D)
$jaPreview = New-UnicodeString -CodePoints @(0x30D3, 0x30E5, 0x30FC)
$jaTranslate = New-UnicodeString -CodePoints @(0x7FFB, 0x8A33)
$arSavePreview = New-UnicodeString -CodePoints @(0x0627, 0x0641, 0x062A, 0x062D, 0x0020, 0x0627, 0x0644, 0x0645, 0x0639, 0x0627, 0x064A, 0x0646, 0x0629, 0x0020, 0x0642, 0x0628, 0x0644, 0x0020, 0x0627, 0x0644, 0x062D, 0x0641, 0x0638)
$arCopyTranslated = New-UnicodeString -CodePoints @(0x0646, 0x0633, 0x062E, 0x0020, 0x0627, 0x0644, 0x0646, 0x0635, 0x0020, 0x0627, 0x0644, 0x0645, 0x062A, 0x0631, 0x062C, 0x0645)
$arOpen = New-UnicodeString -CodePoints @(0x0627, 0x0641, 0x062A, 0x062D)
$arPreview = New-UnicodeString -CodePoints @(0x0627, 0x0644, 0x0645, 0x0639, 0x0627, 0x064A, 0x0646, 0x0629)
$arCopy = New-UnicodeString -CodePoints @(0x0646, 0x0633, 0x062E)

$fixtures = @(
  [ordered]@{
    name = "chinese-large"
    image_path = (Join-Path $FixtureDir "chinese-large.png")
    expect_contains = @($zhSavePreview, $zhCopyTranslated)
    min_blocks = 2
  },
  [ordered]@{
    name = "english-ui"
    image_path = (Join-Path $FixtureDir "english-ui.png")
    expect_contains = @("Open preview before saving", "Copy translated text")
    min_blocks = 2
  },
  [ordered]@{
    name = "technical-small"
    image_path = (Join-Path $FixtureDir "technical-small.png")
    expect_contains = @("PATH=C:\Windows\System32", "LocalModel.exe --help")
    min_blocks = 2
  },
  [ordered]@{
    name = "korean-ui"
    image_path = (Join-Path $FixtureDir "korean-ui.png")
    expect_contains = @($koSave, $koPreview, $koCopy)
    min_blocks = 3
  },
  [ordered]@{
    name = "japanese-ui"
    image_path = (Join-Path $FixtureDir "japanese-ui.png")
    expect_contains = @($jaSave, $jaPreview, $jaTranslate)
    min_blocks = 2
  },
  [ordered]@{
    name = "arabic-ui"
    image_path = (Join-Path $FixtureDir "arabic-ui.png")
    expect_contains = @($arOpen, $arPreview, $arCopy)
    min_blocks = 4
  }
)

if ($RealScreenshotPath) {
  $resolvedRealScreenshotPath = (Resolve-Path -LiteralPath $RealScreenshotPath).Path
  $fixtures += [ordered]@{
    name = "real-screenshot"
    image_path = $resolvedRealScreenshotPath
    expect_contains = @($RealExpectContains)
    min_blocks = $RealMinBlocks
  }
}

if (-not $SkipUserScreenshots -and (Test-Path -LiteralPath $UserScreenshotDir)) {
  $userFixtureExpectations = @{
    "1.png" = [ordered]@{
      expect_contains = @("Developer Community", "Read this before posting", "GPT 5.5")
      min_blocks = 50
    }
    "2.png" = [ordered]@{
      expect_contains = @("best OpenAl model", "human-sounding", "experimenting")
      min_blocks = 15
    }
    "3.png" = [ordered]@{
      expect_contains = @("Intelligent", "Community", "graph-search")
      min_blocks = 6
    }
    "4.png" = [ordered]@{
      expect_contains = @("Read aloud", "Perceived Drop", "MCP tool calls", "Codex credits")
      min_blocks = 14
    }
  }

  Get-ChildItem -LiteralPath $UserScreenshotDir -Filter "*.png" | Sort-Object Name | ForEach-Object {
    $expectation = $userFixtureExpectations[$_.Name]
    if ($expectation) {
      $fixtures += [ordered]@{
        name = "user-$($_.BaseName)"
        image_path = $_.FullName
        expect_contains = @($expectation.expect_contains)
        min_blocks = [int]$expectation.min_blocks
      }
    }
  }

  $userScreenshotTest2Dir = Join-Path $UserScreenshotDir (New-UnicodeString -CodePoints @(0x6D4B, 0x8BD5, 0x0032))
  if (Test-Path -LiteralPath $userScreenshotTest2Dir) {
    $originalTextName = (New-UnicodeString -CodePoints @(0x539F, 0x59CB, 0x6587, 0x672C)) + ".png"
    $wechatTranslationName = (New-UnicodeString -CodePoints @(0x5FAE, 0x4FE1, 0x7FFB, 0x8BD1, 0x7ED3, 0x679C)) + ".png"
    $ourTranslationName = (New-UnicodeString -CodePoints @(0x6211, 0x4EEC, 0x7684, 0x622A, 0x56FE, 0x7FFB, 0x8BD1, 0x7ED3, 0x679C)) + ".png"
    $commercialGrade = New-UnicodeString -CodePoints @(0x5546, 0x4E1A, 0x7EA7)
    $projectPlanFile = New-UnicodeString -CodePoints @(0x9879, 0x76EE, 0x89C4, 0x5212, 0x6587, 0x4EF6)
    $endToEndCommercial = New-UnicodeString -CodePoints @(0x5B8C, 0x6210, 0x7AEF, 0x5230, 0x7AEF, 0x5546, 0x4E1A, 0x7EA7)

    $userTest2FixtureExpectations = @{
      $originalTextName = [ordered]@{
        expect_contains = @("Complete", "OCR/translation", "fixtures/build/smoke", "project planning")
        min_blocks = 35
      }
      $wechatTranslationName = [ordered]@{
        expect_contains = @($commercialGrade, "OCR", $projectPlanFile)
        min_blocks = 5
      }
      $ourTranslationName = [ordered]@{
        expect_contains = @($endToEndCommercial, $projectPlanFile)
        min_blocks = 7
      }
    }

    Get-ChildItem -LiteralPath $userScreenshotTest2Dir -Filter "*.png" | Sort-Object Name | ForEach-Object {
      $expectation = $userTest2FixtureExpectations[$_.Name]
      if ($expectation) {
        $fixtures += [ordered]@{
          name = "user-test2-$($_.BaseName)"
          image_path = $_.FullName
          expect_contains = @($expectation.expect_contains)
          min_blocks = [int]$expectation.min_blocks
        }
      }
    }
  }
}

New-OcrFixtureImage -Path $fixtures[0].image_path -Lines @($zhSavePreview, $zhCopyTranslated) -FontName "Microsoft YaHei" -FontSize 32 -Width 760 -Height 230
New-OcrFixtureImage -Path $fixtures[1].image_path -Lines @("Open preview before saving", "Copy translated text") -FontName "Segoe UI" -FontSize 28 -Width 900 -Height 230
New-OcrFixtureImage -Path $fixtures[2].image_path -Lines @("PATH=C:\Windows\System32", "LocalModel.exe --help") -FontName "Consolas" -FontSize 20 -Width 900 -Height 210
New-OcrFixtureImage -Path $fixtures[3].image_path -Lines @($koSavePreview, $koCopyTranslated) -FontName "Malgun Gothic" -FontSize 28 -Width 1000 -Height 240
New-OcrFixtureImage -Path $fixtures[4].image_path -Lines @($jaSavePreview, $jaCopyTranslated) -FontName "Yu Gothic" -FontSize 28 -Width 1000 -Height 240
New-OcrFixtureImage -Path $fixtures[5].image_path -Lines @($arSavePreview, $arCopyTranslated) -FontName "Arial" -FontSize 28 -Width 1000 -Height 240

Write-Host "Generated RapidOCR fixtures:" -ForegroundColor Cyan
Get-ChildItem -LiteralPath $FixtureDir -Filter *.png | Sort-Object Name | ForEach-Object {
  Write-Host " - $($_.Name) ($($_.Length) bytes)"
}

try {
  foreach ($fixture in $fixtures) {
    $started = Get-Date
    $result = Invoke-RapidOcrFixture -ImagePath $fixture.image_path
    if ($result.status -ne "success") {
      throw "RapidOCR fixture '$($fixture.name)' failed: $($result.error)"
    }
    $blocks = @($result.blocks)
    $joinedText = (($blocks | ForEach-Object { $_.text }) -join "`n")
    $elapsed = [Math]::Round(((Get-Date) - $started).TotalMilliseconds)
    Write-Host "Fixture $($fixture.name): $($blocks.Count) blocks in ${elapsed}ms, lang=$($result.selectedLang)" -ForegroundColor Cyan
    Write-Host $joinedText
    if ($blocks.Count -lt $fixture.min_blocks) {
      throw "Fixture '$($fixture.name)' expected at least $($fixture.min_blocks) blocks, got $($blocks.Count)"
    }
    $normalizedText = Normalize-OcrText -Text $joinedText
    foreach ($expected in $fixture.expect_contains) {
      if (-not $expected) { continue }
      $normalizedExpected = Normalize-OcrText -Text $expected
      if (-not $normalizedText.Contains($normalizedExpected)) {
        throw "Fixture '$($fixture.name)' expected text containing '$expected', got: $joinedText"
      }
    }
  }
} finally {
  if (-not $KeepFixtures) {
    Remove-Item -LiteralPath $FixtureDir -Recurse -Force -ErrorAction SilentlyContinue
  } else {
    Write-Host "RapidOCR fixtures kept at: $FixtureDir" -ForegroundColor Yellow
  }
}

Write-Host "RapidOCR fixture smoke completed." -ForegroundColor Green
