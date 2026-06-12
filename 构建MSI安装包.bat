@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ROOT=%~dp0"
set "CLIENT_DIR=%ROOT%tauri-client"
set "TAURI_DIR=%CLIENT_DIR%\src-tauri"
set "APP_EXE_NAME=YsnTrans"
set "BUNDLE_DIR=%TAURI_DIR%\target\release\bundle\msi"
set "NO_PAUSE="
set "DRY_RUN="

call :parse_args %*

echo(=== YSN Trans - build MSI installer ===
echo(

call :check_project || goto :fail
call :kill_running || goto :fail

echo([cmd] cd /d "%CLIENT_DIR%"
echo([cmd] npm run tauri build -- --bundles msi
echo(

if defined DRY_RUN goto :done

pushd "%CLIENT_DIR%" >nul
call npm run tauri build -- --bundles msi
set "BUILD_CODE=%errorlevel%"
popd >nul

if not "%BUILD_CODE%"=="0" (
  echo(
  echo([FAIL] Build failed with code: %BUILD_CODE%
  exit /b %BUILD_CODE%
)

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
exit /b 0

:kill_running
echo([prepare] Closing running app processes ...
taskkill /F /T /IM %APP_EXE_NAME%.exe >nul 2>nul
if %errorlevel% equ 0 echo([prepare] Closed %APP_EXE_NAME%.exe
taskkill /F /T /IM tauri-client.exe >nul 2>nul
if %errorlevel% equ 0 echo([prepare] Closed legacy tauri-client.exe
echo(
exit /b 0

:fail
set "EXIT_CODE=%errorlevel%"
echo(
echo([FAIL] Script did not complete. Exit code: %EXIT_CODE%
if not defined NO_PAUSE pause
exit /b %EXIT_CODE%

:done
echo([done] MSI output directory:
echo(       %BUNDLE_DIR%
if not defined NO_PAUSE pause
exit /b 0
