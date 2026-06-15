param(
  [string]$SshHost = "n100",
  [string]$RemoteDir = "/home/ysn/screenshot-translator-server",
  [string]$LanBaseUrl = "http://192.168.1.3:8318",
  [string]$PublicBaseUrl = "https://ocr.yousn.me",
  [int]$Port = 8318,
  [switch]$SkipPublicSmoke
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $scriptDir "..\..")).Path

function Invoke-Remote {
  param([string]$Command)
  & ssh $SshHost $Command
  if ($LASTEXITCODE -ne 0) {
    throw "Remote command failed with exit code $LASTEXITCODE"
  }
}

function Copy-RemoteFile {
  param([string]$LocalPath, [string]$RemotePath)
  & scp $LocalPath "${SshHost}:$RemotePath"
  if ($LASTEXITCODE -ne 0) {
    throw "scp failed for $LocalPath"
  }
}

function Invoke-TranslateSmoke {
  param([string]$BaseUrl, [string]$Label)
  $configPath = Join-Path $env:LOCALAPPDATA "ScreenshotTranslator\config.json"
  $config = Get-Content -Raw $configPath | ConvertFrom-Json
  $headers = @{ "x-api-key" = $config.clientToken }
  $text = "Open $Label deployment smoke before saving $([DateTimeOffset]::Now.ToUnixTimeMilliseconds())"
  $body = @{
    source_lang = "en"
    target_lang = "zh"
    render_mode = "client"
    blocks = @(@{
      text = $text
      confidence = 0.99
      box = @(@(0, 0), @(420, 0), @(420, 24), @(0, 24))
    })
  } | ConvertTo-Json -Depth 8

  $watch = [Diagnostics.Stopwatch]::StartNew()
  $result = Invoke-RestMethod -Uri "$BaseUrl/api/translate_text" -Method Post -Headers $headers -ContentType "application/json" -Body $body -TimeoutSec 20
  $watch.Stop()
  $timings = $result.timings
  Write-Host ("[{0}] client={1}ms total={2} provider={3} cache={4} miss={5} text={6}" -f $Label, $watch.ElapsedMilliseconds, $timings.total_ms, $timings.provider_ms, $timings.cache_hits, $timings.provider_misses, $result.translations[0])
}

$remoteBackup = @"
cd '$RemoteDir' &&
ts=`$(date +%Y%m%d-%H%M%S) &&
mkdir -p deploy-backups/`$ts &&
cp app.py config.py http_client.py safe_transport.py security.py translator.py translation_prompt.py requirements.txt translationGlossary.json deploy-backups/`$ts/ 2>/dev/null || true &&
echo deploy-backups/`$ts
"@

Write-Host "[1/6] Backing up remote service files..."
$remoteBackup = $remoteBackup -replace "`r", ""
Invoke-Remote $remoteBackup

Write-Host "[2/6] Uploading complete translation server runtime..."
Copy-RemoteFile (Join-Path $projectRoot "server\app.py") "$RemoteDir/app.py"
Copy-RemoteFile (Join-Path $projectRoot "server\config.py") "$RemoteDir/config.py"
Copy-RemoteFile (Join-Path $projectRoot "server\http_client.py") "$RemoteDir/http_client.py"
Copy-RemoteFile (Join-Path $projectRoot "server\safe_transport.py") "$RemoteDir/safe_transport.py"
Copy-RemoteFile (Join-Path $projectRoot "server\security.py") "$RemoteDir/security.py"
Copy-RemoteFile (Join-Path $projectRoot "server\translator.py") "$RemoteDir/translator.py"
Copy-RemoteFile (Join-Path $projectRoot "server\translation_prompt.py") "$RemoteDir/translation_prompt.py"
Copy-RemoteFile (Join-Path $projectRoot "server\requirements.txt") "$RemoteDir/requirements.txt"
Copy-RemoteFile (Join-Path $projectRoot "tauri-client\src\utils\translationGlossary.json") "$RemoteDir/translationGlossary.json"

Write-Host "[3/6] Running remote syntax and transport-policy checks..."
Invoke-Remote "cd '$RemoteDir' && .venv/bin/python -m py_compile app.py config.py http_client.py safe_transport.py security.py translator.py translation_prompt.py"
Invoke-Remote "cd '$RemoteDir' && .venv/bin/python -c \`"from http_client import get_official_translation_session; from safe_transport import SSRFSafeAdapter; s=get_official_translation_session(); assert s.trust_env is True; assert not isinstance(s.get_adapter('https://'), SSRFSafeAdapter); print('official translation transport: proxy-compatible, unpinned')\`""

Write-Host "[4/6] Restarting uvicorn on port $Port..."
$stopCommand = @"
pids=`$(ps -ef | awk '/[u]vicorn app:app/ && /--port $Port/ {print `$2}')
if [ -n "`$pids" ]; then echo "`$pids" | xargs -r kill; fi
for i in `$(seq 1 20); do
  remaining=`$(ps -ef | awk '/[u]vicorn app:app/ && /--port $Port/ {print `$2}')
  if [ -z "`$remaining" ]; then exit 0; fi
  sleep 0.5
done
ps -ef | awk '/[u]vicorn app:app/ && /--port $Port/ {print}'
exit 1
"@
$stopCommand = $stopCommand -replace "`r", ""
Invoke-Remote $stopCommand
Start-Sleep -Seconds 1
$startCommand = "cd '$RemoteDir' && setsid -f .venv/bin/python -m uvicorn app:app --host 0.0.0.0 --port $Port > uvicorn.log 2>&1 < /dev/null"
$startProcess = Start-Process -FilePath "ssh" -ArgumentList @($SshHost, $startCommand) -WindowStyle Hidden -PassThru
Start-Sleep -Seconds 4
if (-not $startProcess.HasExited) {
  Stop-Process -Id $startProcess.Id -Force
}
Invoke-Remote "ps -ef | grep uvicorn | grep $Port | grep -v grep && tail -20 '$RemoteDir/uvicorn.log'"

Write-Host "[5/6] Checking LAN health..."
$health = Invoke-WebRequest -Uri "$LanBaseUrl/api/health" -TimeoutSec 8 -UseBasicParsing
Write-Host "LAN health: $($health.StatusCode) $($health.Content)"
$healthData = $health.Content | ConvertFrom-Json
if (-not $healthData.translation.glossary_loaded) {
  throw "Translation glossary is not loaded on $LanBaseUrl"
}
Write-Host "Translation glossary: $($healthData.translation.glossary_version), terms=$($healthData.translation.glossary_terms)"

Write-Host "[6/6] Running timing smoke..."
Invoke-TranslateSmoke -BaseUrl $LanBaseUrl -Label "LAN"
if (-not $SkipPublicSmoke) {
  Invoke-TranslateSmoke -BaseUrl $PublicBaseUrl -Label "PUBLIC"
}

Write-Host "N100 translation service deploy completed."
