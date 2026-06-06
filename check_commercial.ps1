param(
  [switch]$TauriBuild,
  [switch]$SmokeLaunch,
  [switch]$OcrFixtures,
  [switch]$SelfTestNativeFailureTrap
)

$ErrorActionPreference = "Stop"

$env:PYTHONIOENCODING = "utf-8"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$clientRoot = Join-Path $repoRoot "tauri-client"
$tauriRoot = Join-Path $clientRoot "src-tauri"

function Run-Step {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Command
  )

  Write-Host "`n==> $Name" -ForegroundColor Cyan
  $startedAt = Get-Date
  & $Command
  $elapsed = (Get-Date) - $startedAt
  Write-Host "OK: $Name ($([Math]::Round($elapsed.TotalSeconds, 1))s)" -ForegroundColor Green
}

function Run-Native {
  param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
  )

  & $FilePath @Arguments
  $exitCode = $LASTEXITCODE
  if ($exitCode -ne 0) {
    $commandLine = @($FilePath) + $Arguments -join " "
    throw "Native command failed with exit code ${exitCode}: ${commandLine}"
  }
}

if ($SelfTestNativeFailureTrap) {
  Write-Host "`n==> native failure trap self-test" -ForegroundColor Cyan
  try {
    Run-Native cmd /c exit 7
    throw "Run-Native did not throw for a failing native command."
  } catch {
    if ($_.Exception.Message -notlike "*Native command failed with exit code 7*") {
      throw
    }
    Write-Host "OK: native failure trap self-test" -ForegroundColor Green
    exit 0
  }
}

Run-Step "i18n dictionary integrity" {
  Push-Location $clientRoot
  try { Run-Native npm run check:i18n } finally { Pop-Location }
}

Run-Step "OCR processing integrity" {
  Push-Location $clientRoot
  try { Run-Native npm run check:ocr-processing } finally { Pop-Location }
}

Run-Step "frontend production build" {
  Push-Location $clientRoot
  try { Run-Native npm run build } finally { Pop-Location }
}

Run-Step "Rust check" {
  Push-Location $tauriRoot
  try { Run-Native cargo check } finally { Pop-Location }
}

Run-Step "Rust tests" {
  Push-Location $tauriRoot
  try { Run-Native cargo test } finally { Pop-Location }
}

if ($OcrFixtures) {
  Run-Step "OCR fixed crop fixtures" {
    Push-Location $clientRoot
    try { Run-Native npm run check:ocr-fixtures } finally { Pop-Location }
  }
} else {
  Write-Host "`nSkipped OCR fixed crop fixtures. Re-run with -OcrFixtures when local OCR models are installed." -ForegroundColor Yellow
}

if ($TauriBuild) {
  Run-Step "Tauri release build" {
    Push-Location $clientRoot
    try { Run-Native npm run tauri -- build } finally { Pop-Location }
  }
} else {
  Write-Host "`nSkipped Tauri release build. Re-run with -TauriBuild for installer verification." -ForegroundColor Yellow
}



if ($SmokeLaunch) {
  Run-Step "release exe smoke launch" {
    $exePath = Join-Path $tauriRoot "target\release\YsnTrans.exe"
    if (-not (Test-Path -LiteralPath $exePath)) {
      throw "Release executable not found at $exePath. Run with -TauriBuild first."
    }
    $probePath = Join-Path $env:TEMP "ysn_screenshot_translator\startup_status.json"
    if (Test-Path -LiteralPath $probePath) {
      Remove-Item -LiteralPath $probePath -Force -ErrorAction SilentlyContinue
    }
    $process = Start-Process -FilePath $exePath -PassThru -WindowStyle Hidden
    try {
      Start-Sleep -Seconds 5
      if ($process.HasExited) {
        throw "Release executable exited during smoke launch with code $($process.ExitCode)."
      }
      if (-not (Test-Path -LiteralPath $probePath)) {
        throw "Startup diagnostics probe was not written: $probePath"
      }
      $probe = Get-Content -LiteralPath $probePath -Raw | ConvertFrom-Json
      if (-not $probe.processId) {
        throw "Startup diagnostics probe is missing processId."
      }
    } finally {
      if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
      }
    }
  }
} else {
  Write-Host "Skipped release exe smoke launch. Re-run with -SmokeLaunch after a release build." -ForegroundColor Yellow
}

Write-Host "`nCommercial check completed." -ForegroundColor Green
