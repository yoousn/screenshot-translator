@echo off
chcp 65001 >nul
cd /d "%~dp0"

set "PYEXE="
where python >nul 2>nul && set "PYEXE=python"
if not defined PYEXE ( where py >nul 2>nul && set "PYEXE=py" )
if not defined PYEXE (
    echo [错误] 未检测到 Python。请先安装 Python 3（https://www.python.org/downloads/，安装时勾选 Add to PATH）。
    pause
    exit /b 1
)

%PYEXE% -c "import webview" >nul 2>nul
if errorlevel 1 (
    echo [安装] 首次运行，正在安装界面组件 pywebview …
    %PYEXE% -m pip install pywebview
    if errorlevel 1 (
        echo [错误] pywebview 安装失败，请检查网络后重试。
        pause
        exit /b 1
    )
)

%PYEXE% "%~dp0app.py"
if errorlevel 1 pause
