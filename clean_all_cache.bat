@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ROOT=%~dp0"
set "DRY_RUN="
set "NO_PAUSE="
set "DEEP="
set "PROJECT_ONLY="
set "WINDOWS_ONLY="
set "SKIP_EXPLORER_RESTART="
set "FAILURES=0"

call :parse_args %*

echo(
echo ============================================================
echo  YsnTrans reliable cache cleanup
echo ============================================================
echo(
echo Default cleanup keeps node_modules, release outputs, OCR models,
echo resources, source files, user data, and Git history.
echo Use --deep only when dependencies and release outputs must be removed.
echo(
if defined DRY_RUN echo([MODE] Dry run only. Nothing will be deleted.
if defined PROJECT_ONLY echo([MODE] Project caches only. Windows caches are skipped.
if defined WINDOWS_ONLY echo([MODE] Windows icon/thumbnail caches only.
if defined DEEP echo([MODE] Deep cleanup includes dependencies and release outputs.
echo(

if not defined WINDOWS_ONLY call :clean_project_caches
if not defined PROJECT_ONLY call :clean_windows_caches

echo(
echo ============================================================
if "%FAILURES%"=="0" (
  echo  Cache cleanup completed successfully.
) else (
  echo  Cache cleanup completed with %FAILURES% warning^(s^).
)
echo ============================================================
echo(
echo Build entry: use the EXE-only build batch file.
echo Complete portable entry: build.bat --portable-only --no-launch
echo(

if not defined NO_PAUSE pause
if "%FAILURES%"=="0" exit /b 0
exit /b 1

:parse_args
if "%~1"=="" exit /b 0
if /I "%~1"=="--dry-run" set "DRY_RUN=1"
if /I "%~1"=="/dry-run" set "DRY_RUN=1"
if /I "%~1"=="--no-pause" set "NO_PAUSE=1"
if /I "%~1"=="/no-pause" set "NO_PAUSE=1"
if /I "%~1"=="--deep" set "DEEP=1"
if /I "%~1"=="/deep" set "DEEP=1"
if /I "%~1"=="--project-only" set "PROJECT_ONLY=1"
if /I "%~1"=="/project-only" set "PROJECT_ONLY=1"
if /I "%~1"=="--windows-only" set "WINDOWS_ONLY=1"
if /I "%~1"=="/windows-only" set "WINDOWS_ONLY=1"
if /I "%~1"=="--no-explorer-restart" set "SKIP_EXPLORER_RESTART=1"
if /I "%~1"=="/no-explorer-restart" set "SKIP_EXPLORER_RESTART=1"
shift
goto :parse_args

:clean_project_caches
echo([project] Cleaning generated caches ...
if not defined DRY_RUN (
  taskkill /F /T /IM YsnTrans.exe >nul 2>nul
  taskkill /F /T /IM tauri-client.exe >nul 2>nul
)
call :remove_dir "%ROOT%tauri-client\src-tauri\target" "Rust/Tauri target cache"
call :remove_dir "%ROOT%tauri-client\dist" "Vite production cache"
call :remove_dir "%ROOT%tauri-client\node_modules\.vite" "Vite dependency cache"
call :remove_dir "%ROOT%tmp-runtime-logs" "runtime smoke logs"
call :remove_dir "%ROOT%.codex-analysis" "local Codex analysis artifacts"
call :remove_dir "%ROOT%.superpowers" "local scratch data"
if defined DEEP (
  call :remove_dir "%ROOT%tauri-client\node_modules" "Node dependencies"
  call :remove_dir "%ROOT%release" "release outputs"
  call :remove_file "%ROOT%YsnTrans.exe" "root launcher"
  call :remove_file "%ROOT%YsnTrans.pdb" "root debug symbols"
)
echo(
exit /b 0

:clean_windows_caches
echo([windows] Cleaning icon and thumbnail caches ...
if not exist "%ROOT%refresh_windows_icon_cache.ps1" (
  echo([error] Missing helper: %ROOT%refresh_windows_icon_cache.ps1
  set /a FAILURES+=1
  exit /b 0
)
set "WINDOWS_CACHE_ARGS=-IncludeThumbnailCache"
if defined DRY_RUN set "WINDOWS_CACHE_ARGS=%WINDOWS_CACHE_ARGS% -WhatIf"
if defined SKIP_EXPLORER_RESTART set "WINDOWS_CACHE_ARGS=%WINDOWS_CACHE_ARGS% -SkipExplorerRestart"
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%refresh_windows_icon_cache.ps1" %WINDOWS_CACHE_ARGS%
if errorlevel 1 (
  echo([warn] Windows cache helper reported a partial failure
  set /a FAILURES+=1
)
if not defined DRY_RUN if not defined SKIP_EXPLORER_RESTART (
  tasklist /FI "IMAGENAME eq explorer.exe" 2>nul | find /I "explorer.exe" >nul
  if errorlevel 1 (
    echo([windows] Explorer was not running after cleanup. Starting it now ...
    start "" explorer.exe
  )
)
echo(
exit /b 0

:remove_dir
set "TARGET_PATH=%~1"
set "TARGET_LABEL=%~2"
if not exist "%TARGET_PATH%\" (
  echo([skip] %TARGET_LABEL%
  exit /b 0
)
if defined DRY_RUN (
  echo([dry ] %TARGET_LABEL%: %TARGET_PATH%
  exit /b 0
)
echo([del ] %TARGET_LABEL%: %TARGET_PATH%
rmdir /S /Q "%TARGET_PATH%" >nul 2>nul
if exist "%TARGET_PATH%\" (
  timeout /T 1 /NOBREAK >nul
  rmdir /S /Q "%TARGET_PATH%" >nul 2>nul
)
if exist "%TARGET_PATH%\" (
  echo([warn] Could not fully remove: %TARGET_PATH%
  set /a FAILURES+=1
)
exit /b 0

:remove_file
set "TARGET_PATH=%~1"
set "TARGET_LABEL=%~2"
if not exist "%TARGET_PATH%" (
  echo([skip] %TARGET_LABEL%
  exit /b 0
)
if defined DRY_RUN (
  echo([dry ] %TARGET_LABEL%: %TARGET_PATH%
  exit /b 0
)
echo([del ] %TARGET_LABEL%: %TARGET_PATH%
del /F /Q "%TARGET_PATH%" >nul 2>nul
if exist "%TARGET_PATH%" (
  echo([warn] Could not remove: %TARGET_PATH%
  set /a FAILURES+=1
)
exit /b 0
