@echo off
setlocal EnableExtensions EnableDelayedExpansion

cd /d "%~dp0"

title YsnTrans Cache Cleanup

echo.
echo ============================================================
echo  YsnTrans cache cleanup and Windows shell refresh
echo ============================================================
echo.
echo This script removes temporary build/system caches only.
echo It does NOT remove models, OCR resources, history, config,
echo recordings, node_modules, or source files.
echo.

net session >nul 2>&1
if %errorlevel% neq 0 (
  echo [WARN] Not running as Administrator.
  echo        Windows Temp and some shell cache files may be skipped.
  echo        Right-click this BAT and choose "Run as administrator" for a deeper cleanup.
  echo.
)

call :delete_dir "tauri-client\dist" "Vite production build output"
call :delete_dir "tauri-client\node_modules\.vite" "Vite dependency cache"
call :delete_dir "tauri-client\node_modules\.cache" "Node package cache folder"
call :delete_dir "tauri-client\src-tauri\target\debug\.fingerprint" "Rust debug fingerprint cache"
call :delete_dir "tauri-client\src-tauri\target\debug\build" "Rust debug build script cache"
call :delete_dir "tauri-client\src-tauri\target\debug\incremental" "Rust debug incremental cache"
call :delete_dir "tauri-client\src-tauri\target\release\.fingerprint" "Rust release fingerprint cache"
call :delete_dir "tauri-client\src-tauri\target\release\build" "Rust release build script cache"
call :delete_dir "tauri-client\src-tauri\target\release\incremental" "Rust release incremental cache"
call :delete_dir ".codex-video-frames" "temporary video analysis frames"
call :delete_dir ".codex-video-frames2" "temporary video analysis frames"

call :clean_temp "%TEMP%" "current user temp"
call :clean_temp "%TMP%" "current user tmp"
call :clean_temp "%SystemRoot%\Temp" "Windows temp"

echo.
echo [INFO] Clearing Explorer thumbnail cache...
if exist "%LocalAppData%\Microsoft\Windows\Explorer\thumbcache_*.db" (
  del /f /q "%LocalAppData%\Microsoft\Windows\Explorer\thumbcache_*.db" >nul 2>&1
)

echo [INFO] Clearing Explorer icon cache...
if exist "%LocalAppData%\IconCache.db" del /f /q "%LocalAppData%\IconCache.db" >nul 2>&1
if exist "%LocalAppData%\Microsoft\Windows\Explorer\iconcache_*.db" (
  del /f /q "%LocalAppData%\Microsoft\Windows\Explorer\iconcache_*.db" >nul 2>&1
)

echo [INFO] Refreshing icon cache...
if exist "%SystemRoot%\System32\ie4uinit.exe" "%SystemRoot%\System32\ie4uinit.exe" -show >nul 2>&1

echo.
echo [INFO] Restarting Windows Explorer shell...
taskkill /f /im explorer.exe >nul 2>&1
timeout /t 2 /nobreak >nul
start explorer.exe

echo.
echo ============================================================
echo  Cleanup complete.
echo ============================================================
echo.
echo Recommended next steps:
echo   1. Fully exit any running YsnTrans tray process.
echo   2. Rebuild or restart the app.
echo   3. Retest Alt+A screenshot and visible-main-window capture.
echo.
pause
exit /b 0

:delete_dir
set "target=%~1"
set "label=%~2"
if exist "%target%" (
  echo [DEL ] %label%: %target%
  rmdir /s /q "%target%" >nul 2>&1
  if exist "%target%" (
    echo [WARN] Could not fully remove: %target%
  ) else (
    echo [ OK ] Removed: %target%
  )
) else (
  echo [SKIP] %label%: %target%
)
exit /b 0

:clean_temp
set "tempdir=%~1"
set "label=%~2"
if "%tempdir%"=="" exit /b 0
if not exist "%tempdir%" (
  echo [SKIP] %label%: %tempdir%
  exit /b 0
)
echo [CLEAN] %label%: %tempdir%
for /d %%D in ("%tempdir%\*") do rmdir /s /q "%%~fD" >nul 2>&1
for %%F in ("%tempdir%\*") do del /f /q "%%~fF" >nul 2>&1
exit /b 0
