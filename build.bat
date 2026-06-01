@echo off
chcp 65001 >nul
setlocal

echo === YSN 截图翻译 - 构建脚本 ===
echo.

cd /d "%~dp0"

echo [准备] 强制关闭正在运行的程序 ...
taskkill /F /T /IM tauri-client.exe >nul 2>nul
if %errorlevel% equ 0 (
    echo [准备] 已关闭 tauri-client.exe
) else (
    echo [准备] 未发现正在运行的 tauri-client.exe
)
echo.

echo [准备] 删除旧产物 ...
if exist "%~dp0tauri-client.exe" (
    for /L %%i in (1,1,10) do (
        del /F /Q "%~dp0tauri-client.exe" >nul 2>nul
        if not exist "%~dp0tauri-client.exe" goto old_output_removed
        timeout /t 1 /nobreak >nul
    )
    echo [错误] 无法删除旧产物：%~dp0tauri-client.exe
    echo [提示] 请确认没有杀毒软件、资源管理器或其他程序正在占用该 exe。
    pause
    exit /b 1
) else (
    echo [准备] 未发现旧产物
    goto old_output_done
)
:old_output_removed
echo [准备] 已删除旧产物
:old_output_done
echo.

cd /d "%~dp0tauri-client"

echo [1/2] 执行全量编译 (Vite 前端 + Rust 后端) ...
call npx tauri build --no-bundle
if %errorlevel% neq 0 (
    echo [错误] Tauri 构建失败！
    pause
    exit /b 1
)
echo [1/2] 编译成功！
echo.

echo [2/2] 复制产物 ...
copy /Y "%~dp0tauri-client\src-tauri\target\release\tauri-client.exe" "%~dp0tauri-client.exe" >nul
if %errorlevel% neq 0 (
    echo [错误] 复制产物失败！
    pause
    exit /b 1
)
echo [2/2] 完成！
echo.

echo === 构建成功 ===
echo 产物位置: %~dp0tauri-client.exe
echo.
pause
