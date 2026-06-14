@echo off
setlocal
chcp 65001 >nul
cd /d "%~dp0"

if exist "%~dp0启动部署助手.vbs" (
    start "" wscript.exe "%~dp0启动部署助手.vbs"
    exit /b 0
)

set "PYEXE="
where python >nul 2>nul && set "PYEXE=python"
if not defined PYEXE ( where py >nul 2>nul && set "PYEXE=py" )
if not defined PYEXE (
    echo [ERROR] Python was not found. Please install Python 3 and add it to PATH.
    pause
    exit /b 1
)

%PYEXE% -c "import webview" >nul 2>nul
if errorlevel 1 (
    echo [INSTALL] Installing pywebview...
    %PYEXE% -m pip install pywebview
    if errorlevel 1 (
        echo [ERROR] pywebview install failed.
        pause
        exit /b 1
    )
)

%PYEXE% "%~dp0app.py"
if errorlevel 1 pause
