@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "CLIENT_DIR=%~1"
if not defined CLIENT_DIR (
  echo([error] Missing frontend project directory argument
  exit /b 1
)

if exist "%CLIENT_DIR%\node_modules\.bin\tauri.cmd" (
  echo([prepare] Node dependencies are ready
  exit /b 0
)

where npm >nul 2>nul
if errorlevel 1 (
  echo([error] npm was not found in PATH
  exit /b 1
)

echo([prepare] Local Tauri CLI is missing. Restoring Node dependencies ...
pushd "%CLIENT_DIR%" >nul
if exist "package-lock.json" (
  echo([cmd] npm ci --no-audit --no-fund
  call npm ci --no-audit --no-fund
) else (
  echo([cmd] npm install --no-audit --no-fund
  call npm install --no-audit --no-fund
)
set "INSTALL_CODE=%errorlevel%"
popd >nul

if not "%INSTALL_CODE%"=="0" (
  echo([error] Node dependency restore failed with code: %INSTALL_CODE%
  exit /b %INSTALL_CODE%
)
if not exist "%CLIENT_DIR%\node_modules\.bin\tauri.cmd" (
  echo([error] Node dependency restore completed but local Tauri CLI is still missing
  exit /b 1
)

echo([prepare] Node dependencies restored
exit /b 0
