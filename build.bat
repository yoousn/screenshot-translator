@echo off
chcp 65001 >nul
echo === YSN 截图翻译 - 构建脚本 ===
echo.

cd /d %~dp0tauri-client

echo [1/3] 编译前端 (Vite + TypeScript) ...
call npm run build
if %errorlevel% neq 0 (
    echo [错误] 前端编译失败！
    pause
    exit /b 1
)
echo [1/3] 前端编译成功！
echo.

echo [2/3] 编译 Rust 后端 (Release) ...
call npx tauri build --no-bundle
if %errorlevel% neq 0 (
    echo [错误] Tauri 构建失败！
    pause
    exit /b 1
)
echo [2/3] Rust 编译成功！
echo.

echo [3/3] 复制产物 & 清理 ...
copy /Y src-tauri\target\release\tauri-client.exe ..\tauri-client.exe >nul
if exist dist (rmdir /s /q dist)
echo [3/3] 完成！
echo.

echo === 构建成功 ===
echo 产物位置: %~dp0tauri-client.exe
echo.
pause
