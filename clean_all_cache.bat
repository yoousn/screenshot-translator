@echo off
setlocal EnableExtensions DisableDelayedExpansion

cd /d "%~dp0"

set "ROOT=%~dp0"
set "DRY_RUN="
set "NO_PAUSE="

for %%A in (%*) do (
  if /I "%%~A"=="--dry-run" set "DRY_RUN=1"
  if /I "%%~A"=="/dry-run" set "DRY_RUN=1"
  if /I "%%~A"=="--no-pause" set "NO_PAUSE=1"
  if /I "%%~A"=="/no-pause" set "NO_PAUSE=1"
)

title YsnTrans Source Slim Cleanup

echo.
echo ============================================================
echo  YsnTrans source slim cleanup
echo ============================================================
echo.
echo This removes regenerable project artifacts only:
echo   release, target, dist, node_modules, temporary runtime logs,
echo   root-level old exe/pdb files, .codex-analysis, and .superpowers.
echo.
echo It does NOT remove OCR models, RapidOCR resources, FFmpeg resources,
echo Git history, user config, recordings, source files, or Windows caches.
echo.
if defined DRY_RUN (
  echo [MODE] Dry run only. Nothing will be deleted.
  echo.
)

if exist "release\" (
  if defined DRY_RUN (echo [DRY ] portable release output: release) else (echo [DEL ] portable release output: release & rmdir /s /q "release" >nul 2>&1)
) else echo [SKIP] portable release output: release

if exist "tauri-client\src-tauri\target\" (
  if defined DRY_RUN (echo [DRY ] Rust/Tauri target output: tauri-client\src-tauri\target) else (echo [DEL ] Rust/Tauri target output: tauri-client\src-tauri\target & rmdir /s /q "tauri-client\src-tauri\target" >nul 2>&1)
) else echo [SKIP] Rust/Tauri target output: tauri-client\src-tauri\target

if exist "tauri-client\dist\" (
  if defined DRY_RUN (echo [DRY ] Vite production build output: tauri-client\dist) else (echo [DEL ] Vite production build output: tauri-client\dist & rmdir /s /q "tauri-client\dist" >nul 2>&1)
) else echo [SKIP] Vite production build output: tauri-client\dist

if exist "tauri-client\node_modules\" (
  if defined DRY_RUN (echo [DRY ] Node dependencies: tauri-client\node_modules) else (echo [DEL ] Node dependencies: tauri-client\node_modules & rmdir /s /q "tauri-client\node_modules" >nul 2>&1)
) else echo [SKIP] Node dependencies: tauri-client\node_modules

if exist "tmp-runtime-logs\" (
  if defined DRY_RUN (echo [DRY ] runtime smoke logs: tmp-runtime-logs) else (echo [DEL ] runtime smoke logs: tmp-runtime-logs & rmdir /s /q "tmp-runtime-logs" >nul 2>&1)
) else echo [SKIP] runtime smoke logs: tmp-runtime-logs

if exist ".codex-analysis\" (
  if defined DRY_RUN (echo [DRY ] local Codex analysis artifacts: .codex-analysis) else (echo [DEL ] local Codex analysis artifacts: .codex-analysis & rmdir /s /q ".codex-analysis" >nul 2>&1)
) else echo [SKIP] local Codex analysis artifacts: .codex-analysis

if exist ".superpowers\" (
  if defined DRY_RUN (echo [DRY ] local Superpowers scratch data: .superpowers) else (echo [DEL ] local Superpowers scratch data: .superpowers & rmdir /s /q ".superpowers" >nul 2>&1)
) else echo [SKIP] local Superpowers scratch data: .superpowers

set "FOUND_TMP_LOG="
for /d %%D in ("%ROOT%tmp-runtime-logs*") do (
  if exist "%%~fD\" (
    set "FOUND_TMP_LOG=1"
    if defined DRY_RUN (echo [DRY ] root temporary runtime log directory: %%~fD) else (echo [DEL ] root temporary runtime log directory: %%~fD & rmdir /s /q "%%~fD" >nul 2>&1)
  )
)
for %%F in ("%ROOT%tmp-runtime-logs*") do (
  if exist "%%~fF" if not exist "%%~fF\" (
    set "FOUND_TMP_LOG=1"
    if defined DRY_RUN (echo [DRY ] root temporary runtime log file: %%~fF) else (echo [DEL ] root temporary runtime log file: %%~fF & del /f /q "%%~fF" >nul 2>&1)
  )
)
if not defined FOUND_TMP_LOG echo [SKIP] root temporary runtime logs: tmp-runtime-logs*

set "FOUND_ROOT_EXE="
for %%F in ("%ROOT%*.exe") do (
  if exist "%%~fF" (
    set "FOUND_ROOT_EXE=1"
    if defined DRY_RUN (echo [DRY ] root-level legacy executable: %%~fF) else (echo [DEL ] root-level legacy executable: %%~fF & del /f /q "%%~fF" >nul 2>&1)
  )
)
if not defined FOUND_ROOT_EXE echo [SKIP] root-level legacy executables: *.exe

set "FOUND_ROOT_PDB="
for %%F in ("%ROOT%*.pdb") do (
  if exist "%%~fF" (
    set "FOUND_ROOT_PDB=1"
    if defined DRY_RUN (echo [DRY ] root-level debug symbol file: %%~fF) else (echo [DEL ] root-level debug symbol file: %%~fF & del /f /q "%%~fF" >nul 2>&1)
  )
)
if not defined FOUND_ROOT_PDB echo [SKIP] root-level debug symbol files: *.pdb

echo.
echo ============================================================
echo  Source slim cleanup complete.
echo ============================================================
echo.
echo Notes:
echo   - Run "cd tauri-client && npm install" after deleting node_modules.
echo   - Run "cd tauri-client && npm run build:rapidocr-runner" only if OCR assets are missing.
echo   - Run ".\build.bat --no-pause" to rebuild app outputs.
echo.

if not defined NO_PAUSE pause
exit /b 0
