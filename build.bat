@echo off
chcp 65001 >nul
echo === YSN 截图翻译 - 构建脚本 ===
echo.

cd /d %~dp0tauri-client

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
copy /Y src-tauri\target\release\tauri-client.exe ..\tauri-client.exe >nul
echo [2/2] 完成！
echo.

echo === 构建成功 ===
echo 产物位置: %~dp0tauri-client.exe
echo.
pause
