param(
  [Parameter(Mandatory = $true)]
  [string] $TargetPath,

  [string] $WorkingDirectory,

  [Parameter(Mandatory = $true)]
  [string] $ShortcutPath,

  [string] $Description = "Open latest YsnTrans build"
)

$ErrorActionPreference = "Stop"

$resolvedTarget = Resolve-Path -LiteralPath $TargetPath
if ([string]::IsNullOrWhiteSpace($WorkingDirectory)) {
  $WorkingDirectory = Split-Path -Parent $resolvedTarget.Path
}
$resolvedWorkingDirectory = Resolve-Path -LiteralPath $WorkingDirectory

$shortcutDirectory = Split-Path -Parent $ShortcutPath
if (-not [string]::IsNullOrWhiteSpace($shortcutDirectory) -and -not (Test-Path -LiteralPath $shortcutDirectory)) {
  New-Item -ItemType Directory -Path $shortcutDirectory | Out-Null
}

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($ShortcutPath)
$shortcut.TargetPath = $resolvedTarget.Path
$shortcut.WorkingDirectory = $resolvedWorkingDirectory.Path
$shortcut.Description = $Description
$shortcut.IconLocation = "$($resolvedTarget.Path),0"
$shortcut.Save()

Write-Output $ShortcutPath
