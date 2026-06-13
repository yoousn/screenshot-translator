@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ROOT=%~dp0"
set "CLIENT_DIR=%ROOT%tauri-client"
set "TAURI_DIR=%CLIENT_DIR%\src-tauri"
set "PORTABLE_DIR=%ROOT%release\YSN-Screenshot-Translator"
set "ARTIFACT_ROOT=%ROOT%build"
set "APP_EXE_NAME=YsnTrans"
set "TARGET_EXE=%TAURI_DIR%\target\release\%APP_EXE_NAME%.exe"
set "SHORTCUT_PATH=%ROOT%%APP_EXE_NAME%.lnk"
set "LEGACY_ROOT_EXE=%ROOT%tauri-client.exe"
set "LEGACY_PORTABLE_EXE=%PORTABLE_DIR%\tauri-client.exe"
set "NO_PAUSE="
set "AUTO_LAUNCH=1"
set "INSTALLERS=1"
set "APP_VERSION=dev"
set "INSTALLER_OUTPUT_DIR="

call :parse_args %*
call :resolve_version || goto :fail
set "INSTALLER_OUTPUT_DIR=%ARTIFACT_ROOT%\x64_v%APP_VERSION%"

echo(=== YSN Screenshot Translator - build ===
echo(

call :kill_running || goto :fail
call :check_inputs || goto :fail
call :ensure_node_dependencies || goto :fail
call :sync_icons || goto :fail
call :prepare_output || goto :fail
call :build_tauri || goto :fail
call :copy_installers || goto :fail
call :copy_portable || goto :fail
call :build_launcher || goto :fail
call :create_root_shortcut || goto :fail
call :launch_portable || goto :fail

echo(
echo(=== Build succeeded ===
echo(Portable directory: %PORTABLE_DIR%
echo(Executable: %PORTABLE_DIR%\%APP_EXE_NAME%.exe
echo(Root launcher: %ROOT%%APP_EXE_NAME%.exe
echo(Root shortcut: %SHORTCUT_PATH%
if defined INSTALLERS echo(Installers: %INSTALLER_OUTPUT_DIR%
echo(Copy the whole portable directory to another computer, not only the exe.
echo(
goto :done

:parse_args
if "%~1"=="" exit /b 0
if /I "%~1"=="--no-pause" set "NO_PAUSE=1"
if /I "%~1"=="/no-pause" set "NO_PAUSE=1"
if /I "%~1"=="--no-launch" set "AUTO_LAUNCH="
if /I "%~1"=="/no-launch" set "AUTO_LAUNCH="
if /I "%~1"=="--no-installers" set "INSTALLERS="
if /I "%~1"=="/no-installers" set "INSTALLERS="
if /I "%~1"=="--portable-only" set "INSTALLERS="
if /I "%~1"=="/portable-only" set "INSTALLERS="
shift
goto :parse_args

:resolve_version
for /f "usebackq delims=" %%V in (`powershell -NoProfile -ExecutionPolicy Bypass -Command "try { ((Get-Content -LiteralPath '%TAURI_DIR%\tauri.conf.json' -Raw) | ConvertFrom-Json).version } catch { '' }"`) do set "APP_VERSION=%%V"
if "%APP_VERSION%"=="" set "APP_VERSION=dev"
set "APP_VERSION=%APP_VERSION: =_%"
echo([prepare] Build artifact version: %APP_VERSION%
echo(
exit /b 0

:fail
set "EXIT_CODE=%errorlevel%"
echo(
echo([FAIL] Build did not complete. Exit code: %EXIT_CODE%
if not defined NO_PAUSE pause
exit /b %EXIT_CODE%

:kill_running
echo([prepare] Closing running app processes ...
taskkill /F /T /IM %APP_EXE_NAME%.exe >nul 2>nul
if %errorlevel% equ 0 (
  echo([prepare] Closed %APP_EXE_NAME%.exe
) else (
  echo([prepare] No running %APP_EXE_NAME%.exe found
)
taskkill /F /T /IM tauri-client.exe >nul 2>nul
if %errorlevel% equ 0 (
  echo([prepare] Closed legacy tauri-client.exe
)
echo(
exit /b 0

:check_inputs
echo([prepare] Checking runtime resources ...
if not exist "%CLIENT_DIR%\package.json" (
  echo([error] Missing frontend project: %CLIENT_DIR%\package.json
  exit /b 1
)
if not exist "%TAURI_DIR%\tauri.conf.json" (
  echo([error] Missing Tauri config: %TAURI_DIR%\tauri.conf.json
  exit /b 1
)
if not exist "%TAURI_DIR%\resources\rapidocr\rapidocr-runner\rapidocr-runner.exe" (
  echo([error] Missing RapidOCR runner:
  echo(        %TAURI_DIR%\resources\rapidocr\rapidocr-runner\rapidocr-runner.exe
  echo([hint] Run: cd /d "%CLIENT_DIR%" ^&^& npm run build:rapidocr-runner
  exit /b 1
)
if not exist "%ROOT%models\rapidocr\ch_PP-OCRv5_det_mobile.onnx" (
  echo([error] Missing RapidOCR models: %ROOT%models\rapidocr
  exit /b 1
)
if not exist "%ROOT%scripts\sync-app-icons.ps1" (
  echo([error] Missing icon sync helper: %ROOT%scripts\sync-app-icons.ps1
  exit /b 1
)
if not exist "%ROOT%scripts\ensure-node-dependencies.bat" (
  echo([error] Missing dependency helper: %ROOT%scripts\ensure-node-dependencies.bat
  exit /b 1
)
echo([prepare] Resource check passed
echo(
exit /b 0

:ensure_node_dependencies
call "%ROOT%scripts\ensure-node-dependencies.bat" "%CLIENT_DIR%"
exit /b %errorlevel%

:sync_icons
echo([prepare] Synchronizing application icons ...
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%scripts\sync-app-icons.ps1" -Source "%TAURI_DIR%\icons\icon.png" -IconsDir "%TAURI_DIR%\icons"
if errorlevel 1 (
  echo([error] Failed to synchronize application icons
  exit /b 1
)
echo(
exit /b 0

:prepare_output
echo([prepare] Cleaning old portable output ...
if exist "%PORTABLE_DIR%" (
  rmdir /S /Q "%PORTABLE_DIR%" >nul 2>nul
  if exist "%PORTABLE_DIR%" (
    echo([hint] Old portable directory could not be fully removed. Reusing it and syncing with robocopy /MIR.
    if exist "%LEGACY_PORTABLE_EXE%" (
      del /F /Q "%LEGACY_PORTABLE_EXE%" >nul 2>nul
      if exist "%LEGACY_PORTABLE_EXE%" (
        echo([error] Old legacy portable exe is still locked: %LEGACY_PORTABLE_EXE%
        exit /b 1
      )
    )
    if exist "%PORTABLE_DIR%\%APP_EXE_NAME%.exe" (
      del /F /Q "%PORTABLE_DIR%\%APP_EXE_NAME%.exe" >nul 2>nul
      if exist "%PORTABLE_DIR%\%APP_EXE_NAME%.exe" (
        echo([error] Old portable exe is still locked: %PORTABLE_DIR%\%APP_EXE_NAME%.exe
        exit /b 1
      )
    )
  )
)
if not exist "%PORTABLE_DIR%" mkdir "%PORTABLE_DIR%" >nul 2>nul
if not exist "%PORTABLE_DIR%" (
  echo([error] Failed to create portable directory: %PORTABLE_DIR%
  exit /b 1
)
if exist "%LEGACY_ROOT_EXE%" (
  del /F /Q "%LEGACY_ROOT_EXE%" >nul 2>nul
  if exist "%LEGACY_ROOT_EXE%" (
    echo([hint] Legacy root tauri-client.exe is locked; skipped it. New output still goes to release.
  ) else (
    echo([prepare] Removed legacy root tauri-client.exe
  )
)
set "ROOT_LAUNCHER_EXE=%ROOT%%APP_EXE_NAME%.exe"
if exist "%ROOT_LAUNCHER_EXE%" (
  del /F /Q "%ROOT_LAUNCHER_EXE%" >nul 2>nul
  if exist "%ROOT_LAUNCHER_EXE%" (
    echo([hint] Root launcher %APP_EXE_NAME%.exe is locked; skipping cleanup.
  ) else (
    echo([prepare] Removed root launcher %APP_EXE_NAME%.exe
  )
)
if exist "%SHORTCUT_PATH%" (
  del /F /Q "%SHORTCUT_PATH%" >nul 2>nul
  if exist "%SHORTCUT_PATH%" (
    echo([hint] Root shortcut is locked; it will be overwritten if possible: %SHORTCUT_PATH%
  ) else (
    echo([prepare] Removed old root shortcut
  )
)
echo(
exit /b 0

:build_tauri
echo([1/4] Building Vite frontend and Rust backend ...
pushd "%CLIENT_DIR%" >nul
if defined INSTALLERS (
  call npx tauri build
) else (
  call npx tauri build --no-bundle
)
set "BUILD_CODE=%errorlevel%"
popd >nul
if not "%BUILD_CODE%"=="0" (
  echo([error] Tauri build failed. Exit code: %BUILD_CODE%
  exit /b %BUILD_CODE%
)
if not exist "%TARGET_EXE%" (
  echo([error] Build finished but exe was not found: %TARGET_EXE%
  exit /b 1
)
echo([1/4] Build completed
echo(
exit /b 0

:copy_installers
if not defined INSTALLERS (
  echo([2/4] Installer bundling disabled
  echo(
  exit /b 0
)
echo([2/4] Copying installers to root build output ...
set "BUNDLE_DIR=%TAURI_DIR%\target\release\bundle"
if not exist "%BUNDLE_DIR%" (
  echo([error] Installer bundle directory was not found: %BUNDLE_DIR%
  exit /b 1
)
if exist "%INSTALLER_OUTPUT_DIR%" (
  rmdir /S /Q "%INSTALLER_OUTPUT_DIR%" >nul 2>nul
  if exist "%INSTALLER_OUTPUT_DIR%" (
    echo([error] Could not clean installer output directory: %INSTALLER_OUTPUT_DIR%
    exit /b 1
  )
)
mkdir "%INSTALLER_OUTPUT_DIR%" >nul 2>nul
if not exist "%INSTALLER_OUTPUT_DIR%" (
  echo([error] Failed to create installer output directory: %INSTALLER_OUTPUT_DIR%
  exit /b 1
)
set "FOUND_INSTALLER="
for /R "%BUNDLE_DIR%" %%F in (*.msi *-setup.exe) do (
  set "FOUND_INSTALLER=1"
  copy /Y "%%~fF" "%INSTALLER_OUTPUT_DIR%\%%~nxF" >nul
  if errorlevel 1 (
    echo([error] Failed to copy installer: %%~fF
    exit /b 1
  )
  echo([2/4] Copied %%~nxF
)
if not defined FOUND_INSTALLER (
  echo([error] No .msi or *-setup.exe installers were found under: %BUNDLE_DIR%
  exit /b 1
)
echo([2/4] Installers are ready: %INSTALLER_OUTPUT_DIR%
echo(
exit /b 0

:copy_portable
echo([3/4] Copying portable runtime files ...
copy /Y "%TARGET_EXE%" "%PORTABLE_DIR%\%APP_EXE_NAME%.exe" >nul
if errorlevel 1 (
  echo([error] Failed to copy exe
  exit /b 1
)
robocopy "%TAURI_DIR%\resources" "%PORTABLE_DIR%\resources" /MIR /NFL /NDL /NJH /NJS /NP >nul
if errorlevel 8 (
  echo([error] Failed to copy resources
  exit /b 1
)
robocopy "%ROOT%models\rapidocr" "%PORTABLE_DIR%\models\rapidocr" /MIR /NFL /NDL /NJH /NJS /NP >nul
if errorlevel 8 (
  echo([error] Failed to copy RapidOCR models
  exit /b 1
)
echo([3/4] Portable directory is ready
echo(
exit /b 0

:build_launcher
echo([4/4] Building root launcher ...
set "LAUNCHER_SRC=%ROOT%scripts\launcher.rs"
set "LAUNCHER_RC=%ROOT%scripts\launcher.rc"
set "LAUNCHER_OBJ=%ROOT%scripts\launcher.o"
set "LAUNCHER_OUT=%ROOT%%APP_EXE_NAME%.exe"

where windres >nul 2>nul
if %errorlevel% equ 0 (
  echo([4/4] Compiling launcher resources ...
  windres -i "%LAUNCHER_RC%" -o "%LAUNCHER_OBJ%" >nul 2>nul
)

if exist "%LAUNCHER_OBJ%" (
  rustc -O --crate-type bin "%LAUNCHER_SRC%" -C link-arg="%LAUNCHER_OBJ%" -o "%LAUNCHER_OUT%" >nul 2>nul
  del /F /Q "%LAUNCHER_OBJ%" >nul 2>nul
) else (
  rustc -O --crate-type bin "%LAUNCHER_SRC%" -o "%LAUNCHER_OUT%" >nul 2>nul
)

if exist "%LAUNCHER_OUT%" (
  echo([4/4] Root launcher compiled successfully
  echo(
  exit /b 0
) else (
  echo([error] Failed to build root launcher
  exit /b 1
)

:create_root_shortcut
set "PORTABLE_EXE=%PORTABLE_DIR%\%APP_EXE_NAME%.exe"
if not exist "%PORTABLE_EXE%" (
  echo([error] Cannot create shortcut for missing executable: %PORTABLE_EXE%
  exit /b 1
)
echo([shortcut] Creating root shortcut ...
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%scripts\create-root-shortcut.ps1" -TargetPath "%PORTABLE_EXE%" -WorkingDirectory "%PORTABLE_DIR%" -ShortcutPath "%SHORTCUT_PATH%" >nul
if errorlevel 1 (
  echo([error] Failed to create root shortcut: %SHORTCUT_PATH%
  exit /b 1
)
echo([shortcut] Root shortcut: %SHORTCUT_PATH%
echo(
exit /b 0

:launch_portable
if not defined AUTO_LAUNCH (
  echo([launch] Auto launch disabled
  echo(
  exit /b 0
)
set "PORTABLE_EXE=%PORTABLE_DIR%\%APP_EXE_NAME%.exe"
if not exist "%PORTABLE_EXE%" (
  echo([error] Cannot launch missing executable: %PORTABLE_EXE%
  exit /b 1
)
echo([launch] Starting %PORTABLE_EXE%
start "" /D "%PORTABLE_DIR%" "%PORTABLE_EXE%"
echo(
exit /b 0

:done
if not defined NO_PAUSE pause
exit /b 0
