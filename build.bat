@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ROOT=%~dp0"
set "CLIENT_DIR=%ROOT%tauri-client"
set "TAURI_DIR=%CLIENT_DIR%\src-tauri"
set "PORTABLE_DIR=%ROOT%release\YSN-Screenshot-Translator"
set "TARGET_EXE=%TAURI_DIR%\target\release\tauri-client.exe"
set "LEGACY_ROOT_EXE=%ROOT%tauri-client.exe"
set "NO_PAUSE="

if /I "%~1"=="--no-pause" set "NO_PAUSE=1"
if /I "%~1"=="/no-pause" set "NO_PAUSE=1"

echo(=== YSN Screenshot Translator - portable build ===
echo(

call :kill_running || goto :fail
call :check_inputs || goto :fail
call :prepare_output || goto :fail
call :build_tauri || goto :fail
call :copy_portable || goto :fail

echo(
echo(=== Build succeeded ===
echo(Portable directory: %PORTABLE_DIR%
echo(Executable: %PORTABLE_DIR%\tauri-client.exe
echo(Copy the whole portable directory to another computer, not only the exe.
echo(
goto :done

:fail
set "EXIT_CODE=%errorlevel%"
echo(
echo([FAIL] Build did not complete. Exit code: %EXIT_CODE%
if not defined NO_PAUSE pause
exit /b %EXIT_CODE%

:kill_running
echo([prepare] Closing running app processes ...
taskkill /F /T /IM tauri-client.exe >nul 2>nul
if %errorlevel% equ 0 (
  echo([prepare] Closed tauri-client.exe
) else (
  echo([prepare] No running tauri-client.exe found
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
echo([prepare] Resource check passed
echo(
exit /b 0

:prepare_output
echo([prepare] Cleaning old portable output ...
if exist "%PORTABLE_DIR%" (
  rmdir /S /Q "%PORTABLE_DIR%" >nul 2>nul
  if exist "%PORTABLE_DIR%" (
    echo([hint] Old portable directory could not be fully removed. Reusing it and syncing with robocopy /MIR.
    if exist "%PORTABLE_DIR%\tauri-client.exe" (
      del /F /Q "%PORTABLE_DIR%\tauri-client.exe" >nul 2>nul
      if exist "%PORTABLE_DIR%\tauri-client.exe" (
        echo([error] Old portable exe is still locked: %PORTABLE_DIR%\tauri-client.exe
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
echo(
exit /b 0

:build_tauri
echo([1/2] Building Vite frontend and Rust backend ...
pushd "%CLIENT_DIR%" >nul
call npx tauri build --no-bundle
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
echo([1/2] Build completed
echo(
exit /b 0

:copy_portable
echo([2/2] Copying portable runtime files ...
copy /Y "%TARGET_EXE%" "%PORTABLE_DIR%\tauri-client.exe" >nul
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
echo([2/2] Portable directory is ready
echo(
exit /b 0

:done
if not defined NO_PAUSE pause
exit /b 0
