param(
  [ValidateSet("scenario4", "all")]
  [string]$Scenario = "scenario4",
  [string]$Root = "C:\Users\ysn\Desktop\zzjt",
  [switch]$KeepApp
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class SmokeNative {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct POINT {
    public int X;
    public int Y;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct WINDOWPLACEMENT {
    public int length;
    public int flags;
    public int showCmd;
    public POINT ptMinPosition;
    public POINT ptMaxPosition;
    public RECT rcNormalPosition;
  }

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool IsIconic(IntPtr hWnd);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool GetWindowPlacement(IntPtr hWnd, ref WINDOWPLACEMENT lpwndpl);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool SetCursorPos(int X, int Y);

  [DllImport("user32.dll")]
  public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
}
"@

$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP = 0x0004
$SW_SHOW = 5
$SW_MINIMIZE = 6
$MINIMIZED_SHOW_CMDS = @(2, 6, 7)

$analysisRoot = Join-Path $Root ".codex-analysis"
New-Item -ItemType Directory -Force -Path $analysisRoot | Out-Null
$runId = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $analysisRoot "ghost-smoke-$runId"
New-Item -ItemType Directory -Force -Path $runDir | Out-Null

$exe = Join-Path $Root "tauri-client\src-tauri\target\debug\YsnTrans.exe"
if (-not (Test-Path -LiteralPath $exe)) {
  throw "Debug exe not found: $exe"
}

function Stop-OldAppProcesses {
  $targets = Get-CimInstance Win32_Process | Where-Object {
    $_.Name -in @("YsnTrans.exe", "tauri-client.exe", "rapidocr-runner.exe") -or
    ($_.CommandLine -and $_.CommandLine -match "com\.screenshot\.translator|YsnTrans\.exe")
  } | Where-Object { $_.ProcessId -ne $PID }

  foreach ($p in $targets) {
    Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
  }
  Start-Sleep -Milliseconds 800
}

function Get-WindowTextValue([IntPtr]$Hwnd) {
  $buffer = New-Object System.Text.StringBuilder 512
  [void][SmokeNative]::GetWindowText($Hwnd, $buffer, $buffer.Capacity)
  $buffer.ToString()
}

function Get-WindowClassValue([IntPtr]$Hwnd) {
  $buffer = New-Object System.Text.StringBuilder 256
  [void][SmokeNative]::GetClassName($Hwnd, $buffer, $buffer.Capacity)
  $buffer.ToString()
}

function Convert-Rect($rect) {
  [pscustomobject]@{
    left = $rect.Left
    top = $rect.Top
    right = $rect.Right
    bottom = $rect.Bottom
    width = $rect.Right - $rect.Left
    height = $rect.Bottom - $rect.Top
  }
}

function Get-WindowPlacementValue([IntPtr]$Hwnd) {
  $placement = New-Object SmokeNative+WINDOWPLACEMENT
  $placement.length = [Runtime.InteropServices.Marshal]::SizeOf([type][SmokeNative+WINDOWPLACEMENT])
  if ([SmokeNative]::GetWindowPlacement($Hwnd, [ref]$placement)) {
    return [pscustomobject]@{
      showCmd = $placement.showCmd
      minimized = $MINIMIZED_SHOW_CMDS -contains $placement.showCmd
      normalRect = Convert-Rect $placement.rcNormalPosition
    }
  }
  return $null
}

function Get-AppWindows([int]$ProcessId) {
  $windows = New-Object System.Collections.Generic.List[object]
  $callback = [SmokeNative+EnumWindowsProc]{
    param([IntPtr]$hwnd, [IntPtr]$lparam)
    $pidOut = 0
    [void][SmokeNative]::GetWindowThreadProcessId($hwnd, [ref]$pidOut)
    if ($pidOut -eq $ProcessId) {
      $rect = New-Object SmokeNative+RECT
      $hasRect = [SmokeNative]::GetWindowRect($hwnd, [ref]$rect)
      $windows.Add([pscustomobject]@{
        hwnd = $hwnd
        hwndValue = $hwnd.ToInt64()
        title = Get-WindowTextValue $hwnd
        className = Get-WindowClassValue $hwnd
        visible = [SmokeNative]::IsWindowVisible($hwnd)
        iconic = [SmokeNative]::IsIconic($hwnd)
        placement = Get-WindowPlacementValue $hwnd
        rect = if ($hasRect) { Convert-Rect $rect } else { $null }
      }) | Out-Null
    }
    return $true
  }
  [void][SmokeNative]::EnumWindows($callback, [IntPtr]::Zero)
  $windows
}

function Find-MainWindow([int]$ProcessId) {
  $windows = Get-AppWindows $ProcessId
  $main = $windows |
    Where-Object { $_.title -eq "YsnTrans" -and $_.rect -and $_.rect.width -gt 300 -and $_.rect.height -gt 200 } |
    Select-Object -First 1
  if (-not $main) {
    $main = $windows |
      Where-Object { $_.title -match "YsnTrans" -and $_.rect -and $_.rect.width -gt 300 -and $_.rect.height -gt 200 } |
      Select-Object -First 1
  }
  $main
}

function Wait-MainWindow([int]$ProcessId, [bool]$Visible, [int]$TimeoutMs = 10000) {
  $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
  do {
    $main = Find-MainWindow $ProcessId
    if ($main -and ($main.visible -eq $Visible)) {
      return $main
    }
    Start-Sleep -Milliseconds 100
  } while ((Get-Date) -lt $deadline)
  return (Find-MainWindow $ProcessId)
}

function Save-DesktopImage([string]$Path) {
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
  $gfx = [System.Drawing.Graphics]::FromImage($bmp)
  try {
    $gfx.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $gfx.Dispose()
    $bmp.Dispose()
  }
}

function Minimize-OtherTopLevelWindows {
  $currentPid = $PID
  $callback = [SmokeNative+EnumWindowsProc]{
    param([IntPtr]$hwnd, [IntPtr]$lparam)
    if (-not [SmokeNative]::IsWindowVisible($hwnd)) { return $true }

    $pidOut = 0
    [void][SmokeNative]::GetWindowThreadProcessId($hwnd, [ref]$pidOut)
    if ($pidOut -eq $currentPid) { return $true }

    $title = Get-WindowTextValue $hwnd
    if ([string]::IsNullOrWhiteSpace($title)) { return $true }

    $className = Get-WindowClassValue $hwnd
    if ($className -in @("Shell_TrayWnd", "Progman", "WorkerW")) { return $true }

    $rect = New-Object SmokeNative+RECT
    if (-not [SmokeNative]::GetWindowRect($hwnd, [ref]$rect)) { return $true }
    if (($rect.Right - $rect.Left) -lt 120 -or ($rect.Bottom - $rect.Top) -lt 80) { return $true }

    [void][SmokeNative]::ShowWindow($hwnd, $SW_MINIMIZE)
    return $true
  }
  [void][SmokeNative]::EnumWindows($callback, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 800
}

function Is-WindowMinimized($Window) {
  if (-not $Window) { return $false }
  if ($Window.iconic) { return $true }
  if ($Window.placement -and $Window.placement.minimized) { return $true }
  return $false
}

function Measure-ImageRegion([string]$Path, $Rect) {
  if (-not $Rect) { return $null }
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = [System.Drawing.Bitmap]::FromFile($Path)
  try {
    $left = [Math]::Max(0, $Rect.left - $bounds.Left)
    $top = [Math]::Max(0, $Rect.top - $bounds.Top)
    $right = [Math]::Min($bmp.Width - 1, $Rect.right - $bounds.Left)
    $bottom = [Math]::Min($bmp.Height - 1, $Rect.bottom - $bounds.Top)
    if ($right -le $left -or $bottom -le $top) { return $null }

    $samples = 0
    $white = 0
    $sum = 0.0
    for ($y = $top; $y -le $bottom; $y += 8) {
      for ($x = $left; $x -le $right; $x += 8) {
        $c = $bmp.GetPixel($x, $y)
        $lum = ($c.R + $c.G + $c.B) / 3.0
        $sum += $lum
        $samples += 1
        if ($c.R -ge 245 -and $c.G -ge 245 -and $c.B -ge 245) { $white += 1 }
      }
    }

    [pscustomobject]@{
      samples = $samples
      whiteRatio = if ($samples -gt 0) { [Math]::Round($white / $samples, 4) } else { 0 }
      mean = if ($samples -gt 0) { [Math]::Round($sum / $samples, 2) } else { 0 }
    }
  } finally {
    $bmp.Dispose()
  }
}

function Click-WindowCloseButton($Window) {
  $x = [int]($Window.rect.right - 18)
  $y = [int]($Window.rect.top + 16)
  [void][SmokeNative]::SetForegroundWindow($Window.hwnd)
  Start-Sleep -Milliseconds 150
  [void][SmokeNative]::SetCursorPos($x, $y)
  Start-Sleep -Milliseconds 80
  [SmokeNative]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [SmokeNative]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
}

function Minimize-Window($Window) {
  $x = [int]($Window.rect.right - 104)
  $y = [int]($Window.rect.top + 16)
  [void][SmokeNative]::SetForegroundWindow($Window.hwnd)
  Start-Sleep -Milliseconds 150
  [void][SmokeNative]::SetCursorPos($x, $y)
  Start-Sleep -Milliseconds 80
  [SmokeNative]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [SmokeNative]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 900
  $placement = Get-WindowPlacementValue $Window.hwnd
  if ((-not [SmokeNative]::IsIconic($Window.hwnd)) -and (-not ($placement -and $placement.minimized))) {
    [void][SmokeNative]::ShowWindow($Window.hwnd, $SW_MINIMIZE)
    Start-Sleep -Milliseconds 700
  }
}

function Send-Keys([string]$Keys) {
  $shell = New-Object -ComObject WScript.Shell
  $shell.SendKeys($Keys)
}

function Wait-TempScreenshot([datetime]$PreviousWrite, [int]$TimeoutMs = 12000) {
  $path = Join-Path $env:LOCALAPPDATA "ScreenshotTranslator\fullscreen_temp.png"
  $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
  do {
    if (Test-Path -LiteralPath $path) {
      $item = Get-Item -LiteralPath $path
      if ($item.Length -gt 1000 -and $item.LastWriteTime -gt $PreviousWrite) {
        return $item
      }
    }
    Start-Sleep -Milliseconds 100
  } while ((Get-Date) -lt $deadline)
  return $null
}

function Start-App {
  $out = Join-Path $runDir "ysntrans-out.log"
  $err = Join-Path $runDir "ysntrans-err.log"
  $proc = Start-Process -FilePath $exe `
    -WorkingDirectory (Split-Path -Parent $exe) `
    -WindowStyle Hidden `
    -RedirectStandardOutput $out `
    -RedirectStandardError $err `
    -PassThru
  Start-Sleep -Milliseconds 2500
  $proc
}

function Show-MainViaSingleInstance([int]$ProcessId) {
  Start-Process -FilePath $exe -WorkingDirectory (Split-Path -Parent $exe) -WindowStyle Hidden | Out-Null
  $main = Wait-MainWindow $ProcessId $true 7000
  if (-not $main -or -not $main.visible) {
    $main = Find-MainWindow $ProcessId
    if ($main) {
      [void][SmokeNative]::ShowWindow($main.hwnd, $SW_SHOW)
      [void][SmokeNative]::SetForegroundWindow($main.hwnd)
      Start-Sleep -Milliseconds 500
      $main = Wait-MainWindow $ProcessId $true 3000
    }
  }
  $main
}

function Run-AltACloseCase([string]$Name, [int]$ProcessId, $RegionRect) {
  $caseDir = Join-Path $runDir $Name
  New-Item -ItemType Directory -Force -Path $caseDir | Out-Null

  $mainBefore = Find-MainWindow $ProcessId
  if (-not $RegionRect -and $mainBefore) {
    $RegionRect = $mainBefore.rect
  }

  $beforePath = Join-Path $caseDir "desktop-before.png"
  Save-DesktopImage $beforePath

  $tempPath = Join-Path $env:LOCALAPPDATA "ScreenshotTranslator\fullscreen_temp.png"
  $previousWrite = [datetime]::MinValue
  if (Test-Path -LiteralPath $tempPath) {
    $previousWrite = (Get-Item -LiteralPath $tempPath).LastWriteTime
  }

  Send-Keys "%a"
  $temp = Wait-TempScreenshot $previousWrite 15000
  $tempCopyPath = Join-Path $caseDir "fullscreen_temp.png"
  if ($temp) {
    Copy-Item -LiteralPath $temp.FullName -Destination $tempCopyPath -Force
  }

  Start-Sleep -Milliseconds 800
  $overlayPath = Join-Path $caseDir "desktop-overlay.png"
  Save-DesktopImage $overlayPath

  Send-Keys "{ESC}"
  Start-Sleep -Milliseconds 1300
  $afterPath = Join-Path $caseDir "desktop-after.png"
  Save-DesktopImage $afterPath

  $mainAfter = Find-MainWindow $ProcessId
  $measureBefore = Measure-ImageRegion $beforePath $RegionRect
  $measureTemp = if (Test-Path -LiteralPath $tempCopyPath) { Measure-ImageRegion $tempCopyPath $RegionRect } else { $null }
  $measureAfter = Measure-ImageRegion $afterPath $RegionRect

  [pscustomobject]@{
    name = $Name
    tempCreated = [bool]$temp
    tempLength = if ($temp) { $temp.Length } else { 0 }
    alive = [bool](Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
    mainBefore = $mainBefore
    mainBeforeMinimized = Is-WindowMinimized $mainBefore
    mainAfter = $mainAfter
    mainAfterMinimized = Is-WindowMinimized $mainAfter
    beforeImage = $beforePath
    tempImage = if (Test-Path -LiteralPath $tempCopyPath) { $tempCopyPath } else { $null }
    overlayImage = $overlayPath
    afterImage = $afterPath
    beforeRegion = $measureBefore
    tempRegion = $measureTemp
    afterRegion = $measureAfter
  }
}

function Run-Scenario1([int]$ProcessId) {
  $main = Wait-MainWindow $ProcessId $false 5000
  Run-AltACloseCase "01_hidden_alt_a_close" $ProcessId $main.rect
}

function Run-Scenario2([int]$ProcessId) {
  $main = Show-MainViaSingleInstance $ProcessId
  if (-not $main) { throw "main window not found for scenario 2" }
  Start-Sleep -Milliseconds 500
  Run-AltACloseCase "02_show_main_alt_a_close" $ProcessId $main.rect
}

function Run-Scenario3([int]$ProcessId) {
  $main = Show-MainViaSingleInstance $ProcessId
  if (-not $main) { throw "main window not found for scenario 3" }
  Start-Sleep -Milliseconds 500
  Minimize-Window $main
  $minimized = Wait-MainWindow $ProcessId $true 5000
  Run-AltACloseCase "03_show_minimize_alt_a_close" $ProcessId $main.rect
}

function Run-Scenario4([int]$ProcessId) {
  $name = "04_show_x_close_alt_a_close"

  $mainShown = Show-MainViaSingleInstance $ProcessId
  if (-not $mainShown) { throw "main window not found" }
  Start-Sleep -Milliseconds 500

  Click-WindowCloseButton $mainShown
  $mainHidden = Wait-MainWindow $ProcessId $false 8000
  Start-Sleep -Milliseconds 500

  Run-AltACloseCase $name $ProcessId $mainShown.rect
}

function Run-Scenario5([int]$ProcessId) {
  $main = Show-MainViaSingleInstance $ProcessId
  if (-not $main) { throw "main window not found for scenario 5" }
  Start-Sleep -Milliseconds 500
  Run-AltACloseCase "05_show_taskbar_restore_alt_a_close" $ProcessId $main.rect
}

Stop-OldAppProcesses
try { $null = (New-Object -ComObject Shell.Application).MinimizeAll() } catch {}
Minimize-OtherTopLevelWindows
Start-Sleep -Milliseconds 700

$proc = Start-App
$mainInitial = Wait-MainWindow $proc.Id $false 12000
if (-not $mainInitial) {
  throw "YsnTrans started but no main window was found for pid $($proc.Id)"
}

$results = @()
if ($Scenario -eq "all") {
  $results += Run-Scenario1 $proc.Id
  $results += Run-Scenario2 $proc.Id
  $results += Run-Scenario3 $proc.Id
  $results += Run-Scenario4 $proc.Id
  $results += Run-Scenario5 $proc.Id
} elseif ($Scenario -eq "scenario4") {
  $results += Run-Scenario4 $proc.Id
}

$summaryPath = Join-Path $runDir "summary.json"
$results | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $summaryPath -Encoding UTF8
$results | ForEach-Object {
  "{0}: tempCreated={1} tempLength={2} alive={3} mainBefore=visible:{4},min:{5},rect:{6},{7},{8},{9} mainAfter=visible:{10},min:{11},rect:{12},{13},{14},{15} tempWhiteRatio={16} tempMean={17} afterWhiteRatio={18} afterMean={19}" -f `
    $_.name, $_.tempCreated, $_.tempLength, $_.alive,
    $_.mainBefore.visible, $_.mainBeforeMinimized, $_.mainBefore.rect.left, $_.mainBefore.rect.top, $_.mainBefore.rect.right, $_.mainBefore.rect.bottom,
    $_.mainAfter.visible, $_.mainAfterMinimized, $_.mainAfter.rect.left, $_.mainAfter.rect.top, $_.mainAfter.rect.right, $_.mainAfter.rect.bottom,
    $_.tempRegion.whiteRatio, $_.tempRegion.mean, $_.afterRegion.whiteRatio, $_.afterRegion.mean
}
"summary=$summaryPath"

if (-not $KeepApp) {
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}
