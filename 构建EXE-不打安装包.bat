@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ROOT=%~dp0"
set "CLIENT_DIR=%ROOT%tauri-client"
set "TAURI_DIR=%CLIENT_DIR%\src-tauri"
set "APP_EXE_NAME=YsnTrans"
set "TARGET_EXE=%TAURI_DIR%\target\release\%APP_EXE_NAME%.exe"
set "SHORTCUT_PATH=%ROOT%%APP_EXE_NAME%.lnk"
set "NO_PAUSE="
set "DRY_RUN="

call :parse_args %*

echo(=== YSN Trans - build exe only ===
echo(

call :check_project || goto :fail

echo([cmd] cd /d "%CLIENT_DIR%"
echo([cmd] npm run tauri build -- --no-bundle
echo([output] %TARGET_EXE%
echo([shortcut] %SHORTCUT_PATH%
echo([launch] %TARGET_EXE%
echo(

if defined DRY_RUN goto :done

call :kill_running || goto :fail

pushd "%CLIENT_DIR%" >nul
call npm run tauri build -- --no-bundle
set "BUILD_CODE=%errorlevel%"
popd >nul

if not "%BUILD_CODE%"=="0" (
  echo(
  echo([FAIL] Build failed with code: %BUILD_CODE%
  exit /b %BUILD_CODE%
)

if not exist "%TARGET_EXE%" (
  echo([error] Build finished but exe was not found:
  echo(        %TARGET_EXE%
  exit /b 1
)

call :create_root_shortcut || goto :fail
call :launch_built_exe || goto :fail

goto :done

:parse_args
if "%~1"=="" exit /b 0
if /I "%~1"=="--no-pause" set "NO_PAUSE=1"
if /I "%~1"=="/no-pause" set "NO_PAUSE=1"
if /I "%~1"=="--dry-run" set "DRY_RUN=1"
if /I "%~1"=="/dry-run" set "DRY_RUN=1"
shift
goto :parse_args

:check_project
if not exist "%CLIENT_DIR%\package.json" (
  echo([error] Missing frontend project: %CLIENT_DIR%\package.json
  exit /b 1
)
if not exist "%TAURI_DIR%\tauri.conf.json" (
  echo([error] Missing Tauri config: %TAURI_DIR%\tauri.conf.json
  exit /b 1
)
where npm >nul 2>nul
if errorlevel 1 (
  echo([error] npm was not found in PATH
  exit /b 1
)
if not exist "%ROOT%scripts\create-root-shortcut.ps1" (
  echo([error] Missing shortcut helper: %ROOT%scripts\create-root-shortcut.ps1
  exit /b 1
)
exit /b 0

:kill_running
echo([prepare] Closing running app processes ...
taskkill /F /T /IM %APP_EXE_NAME%.exe >nul 2>nul
if %errorlevel% equ 0 echo([prepare] Closed %APP_EXE_NAME%.exe
taskkill /F /T /IM tauri-client.exe >nul 2>nul
if %errorlevel% equ 0 echo([prepare] Closed legacy tauri-client.exe
echo(
exit /b 0

:create_root_shortcut
echo([shortcut] Creating root shortcut ...
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%scripts\create-root-shortcut.ps1" -TargetPath "%TARGET_EXE%" -WorkingDirectory "%TAURI_DIR%\target\release" -ShortcutPath "%SHORTCUT_PATH%" >nul
if errorlevel 1 (
  echo([error] Failed to create root shortcut: %SHORTCUT_PATH%
  exit /b 1
)
echo([shortcut] Root shortcut: %SHORTCUT_PATH%
echo(
exit /b 0

:launch_built_exe
echo([launch] Starting %TARGET_EXE%
start "" /D "%TAURI_DIR%\target\release" "%TARGET_EXE%"
echo(
exit /b 0

:fail
set "EXIT_CODE=%errorlevel%"
echo(
echo([FAIL] Script did not complete. Exit code: %EXIT_CODE%
if not defined NO_PAUSE pause
exit /b %EXIT_CODE%

:done
echo([done] Executable output:
echo(       %TARGET_EXE%
echo([done] Root shortcut:
echo(       %SHORTCUT_PATH%
if not defined NO_PAUSE pause
exit /b 0
