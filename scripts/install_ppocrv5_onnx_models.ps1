param(
  [string]$AppDataRoot = (Join-Path (Split-Path -Parent $PSScriptRoot) 'models\ocr')
)

$ErrorActionPreference = 'Stop'

$sourceRoot = Join-Path $AppDataRoot 'source\paddleocr-v5'
$activeRoot = Join-Path $AppDataRoot 'active'
$modelsDir = Join-Path $activeRoot 'models'
$dictDir = Join-Path $activeRoot 'dictionaries'

New-Item -ItemType Directory -Force -Path $sourceRoot, $modelsDir, $dictDir | Out-Null


function Write-JsonUtf8NoBom {
  param(
    [Parameter(Mandatory = $true)]$Value,
    [Parameter(Mandatory = $true)][string]$Path,
    [int]$Depth = 10
  )

  $json = $Value | ConvertTo-Json -Depth $Depth
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $json, $utf8NoBom)
}

function Save-UrlIfMissingOrSmall {
  param(
    [Parameter(Mandatory = $true)][string]$Url,
    [Parameter(Mandatory = $true)][string]$Path,
    [int64]$MinBytes = 1
  )

  $needsDownload = !(Test-Path -LiteralPath $Path)
  if (!$needsDownload) {
    $needsDownload = (Get-Item -LiteralPath $Path).Length -lt $MinBytes
  }

  if ($needsDownload) {
    Write-Host "Downloading $Path"
    curl.exe -L --fail --retry 3 --connect-timeout 20 -o $Path $Url
  } else {
    Write-Host "Already exists $Path"
  }
}

$officialPaddleSources = @(
  @{ Url = 'https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/PP-OCRv5_mobile_det_infer.tar'; Path = (Join-Path $sourceRoot 'PP-OCRv5_mobile_det_infer.tar'); MinBytes = 4000000 },
  @{ Url = 'https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/PP-OCRv5_server_rec_infer.tar'; Path = (Join-Path $sourceRoot 'PP-OCRv5_server_rec_infer.tar'); MinBytes = 80000000 }
)

foreach ($item in $officialPaddleSources) {
  Save-UrlIfMissingOrSmall -Url $item.Url -Path $item.Path -MinBytes $item.MinBytes
}

$hfBase = 'https://huggingface.co/monkt/paddleocr-onnx/resolve/main'
$artifacts = @(
  @{ Kind = 'model'; Id = 'det-default'; RelativePath = 'models/det-default.onnx'; Url = "$hfBase/detection/v5/det.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'cls-default'; RelativePath = 'models/cls-default.onnx'; Url = "$hfBase/preprocessing/textline-orientation/PP-LCNet_x1_0_textline_ori.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'rec-cjk'; RelativePath = 'models/rec-cjk.onnx'; Url = "$hfBase/languages/chinese/rec.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'rec-latin'; RelativePath = 'models/rec-latin.onnx'; Url = "$hfBase/languages/latin/rec.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'rec-korean'; RelativePath = 'models/rec-korean.onnx'; Url = "$hfBase/languages/korean/rec.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'rec-cyrillic'; RelativePath = 'models/rec-cyrillic.onnx'; Url = "$hfBase/languages/eslav/rec.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'rec-arabic'; RelativePath = 'models/rec-arabic.onnx'; Url = "$hfBase/languages/arabic/rec.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'model'; Id = 'rec-thai'; RelativePath = 'models/rec-thai.onnx'; Url = "$hfBase/languages/thai/rec.onnx"; MinBytes = 1000000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'dictionary'; Id = 'rec-cjk'; RelativePath = 'dictionaries/cjk.txt'; Url = "$hfBase/languages/chinese/dict.txt"; MinBytes = 1000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'dictionary'; Id = 'rec-latin'; RelativePath = 'dictionaries/latin.txt'; Url = "$hfBase/languages/latin/dict.txt"; MinBytes = 100; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'dictionary'; Id = 'rec-korean'; RelativePath = 'dictionaries/korean.txt'; Url = "$hfBase/languages/korean/dict.txt"; MinBytes = 1000; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'dictionary'; Id = 'rec-cyrillic'; RelativePath = 'dictionaries/cyrillic.txt'; Url = "$hfBase/languages/eslav/dict.txt"; MinBytes = 100; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'dictionary'; Id = 'rec-arabic'; RelativePath = 'dictionaries/arabic.txt'; Url = "$hfBase/languages/arabic/dict.txt"; MinBytes = 100; License = 'Apache-2.0-reviewed' },
  @{ Kind = 'dictionary'; Id = 'rec-thai'; RelativePath = 'dictionaries/thai.txt'; Url = "$hfBase/languages/thai/dict.txt"; MinBytes = 100; License = 'Apache-2.0-reviewed' }
)

$installed = @()
foreach ($artifact in $artifacts) {
  $relativePath = $artifact.RelativePath.Replace('/', '\')
  $absolutePath = Join-Path $activeRoot $relativePath
  Save-UrlIfMissingOrSmall -Url $artifact.Url -Path $absolutePath -MinBytes $artifact.MinBytes
  $file = Get-Item -LiteralPath $absolutePath
  $sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $absolutePath).Hash.ToLowerInvariant()
  $installed += [PSCustomObject]@{
    kind = $artifact.Kind
    id = $artifact.Id
    relativePath = $artifact.RelativePath
    url = $artifact.Url
    license = $artifact.License
    size = $file.Length
    sha256 = $sha256
  }
}

Write-JsonUtf8NoBom -Value $installed -Depth 4 -Path (Join-Path $activeRoot 'installed-artifacts.json')

$manifestPath = Join-Path $AppDataRoot 'manifest.json'
if (Test-Path -LiteralPath $manifestPath) {
  $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json

  foreach ($pack in $manifest.packs) {
    if ($pack.id -eq 'auto-multilingual-balanced') {
      $pack.status = 'installed'
      if ($pack.PSObject.Properties.Name -contains 'lastError') {
        $pack.PSObject.Properties.Remove('lastError')
      }
    }
  }

  foreach ($model in $manifest.models) {
    $modelArtifact = $installed | Where-Object { $_.kind -eq 'model' -and $_.id -eq $model.id } | Select-Object -First 1
    if ($null -ne $modelArtifact) {
      $model.path = $modelArtifact.relativePath
      $model.size = $modelArtifact.size
      $model.sha256 = $modelArtifact.sha256
      $model.status = 'installed'
      $model.source = [PSCustomObject]@{ provider = 'ysn-managed'; url = $modelArtifact.url; license = $modelArtifact.license }
      if ($model.PSObject.Properties.Name -contains 'lastError') {
        $model.PSObject.Properties.Remove('lastError')
      }
    }

    $dictionaryArtifact = $installed | Where-Object { $_.kind -eq 'dictionary' -and $_.id -eq $model.id } | Select-Object -First 1
    if ($null -ne $dictionaryArtifact -and $null -ne $model.contract -and $null -ne $model.contract.dictionary) {
      $model.contract.dictionary.path = $dictionaryArtifact.relativePath
      $model.contract.dictionary.size = $dictionaryArtifact.size
      $model.contract.dictionary.sha256 = $dictionaryArtifact.sha256
      $model.contract.dictionary.source = [PSCustomObject]@{ provider = 'ysn-managed'; url = $dictionaryArtifact.url; license = $dictionaryArtifact.license }
    }
  }

  if ($null -eq $manifest.installedAt) {
    $manifest.installedAt = (Get-Date).ToString('o')
  }
  $manifest.updatedAt = (Get-Date).ToString('o')
  Write-JsonUtf8NoBom -Value $manifest -Depth 50 -Path $manifestPath
  Write-Host "Updated manifest: $manifestPath"
} else {
  Write-Warning "Manifest not found yet: $manifestPath. Open the app once, then rerun this script to mark the pack installed."
}

Write-Host "PP-OCRv5 ONNX model pack installed: $activeRoot"
$installed | Sort-Object relativePath | Format-Table -AutoSize
