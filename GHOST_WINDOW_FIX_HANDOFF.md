# YsnTrans 幽灵窗口修复交接文档

> 生成时间：2026-06-07  
> 当前状态：暂停继续桌面操作，等待换电脑后继续。  
> 结论先写在最前：本轮已经解决了“截图底图里捕到主面板白块/半透明残影”的大部分路径，但尚未彻底解决“主面板右上角 X 隐藏后，再 Alt+A 截图关闭时，桌面真实出现一个白色 YsnTrans 主窗口壳”的路径。这个文档就是为了换环境后能从同一个点继续。

## 1. 用户要求的固定验收场景

用户明确要求必须测试这些场景：

1. `Alt+A` 截图，然后关闭截图。
2. 打开主面板，`Alt+A` 截图，然后关闭截图。
3. 打开主面板，最小化主面板，`Alt+A` 截图，然后关闭截图。
4. 打开主面板，点击右上角 `X` 退出/隐藏主面板，`Alt+A` 截图，然后关闭截图。
5. 打开主面板；如果主面板不显示，就点击任务栏主面板让它出现；然后 `Alt+A` 截图，关闭截图。

本轮实际按这 5 项做了多轮自动化桌面测试，不是网页内预览。

## 2. 原始问题判断

用户提供的视频：

- `D:/Desktop/SnowShot_Video_2026-06-07_06-01-26.mp4`

我抽帧后观察到的问题形态：

- 视频里不是普通独立窗口一直浮在桌面上。
- 更像是截图流程把 YsnTrans/Tauri/WebView2 主窗口区域捕成白块，叠加截图遮罩后看起来像幽灵窗口。
- 后续真实桌面测试又发现另一个问题：当主窗口已经通过右上角 `X` 隐藏时，关闭截图 overlay 后，Windows 会把隐藏的 Tauri 主窗口壳激活成一个真实白色窗口。这个白窗在桌面上可见，但 Tauri/Win32 枚举里 `IsWindowVisible` 可能仍报告为 `false`，所以不能只相信窗口状态字段。

所以现在至少有两个相关但不完全相同的问题：

- 问题 A：截图底图里出现主面板白块或半透明残影。
- 问题 B：截图关闭后桌面上出现真实白色 YsnTrans 主窗口壳。

本轮进展：

- 问题 A 在可见主面板路径上已明显改善，最新第 2/5 项底图肉眼检查干净。
- 问题 B 在第 4 项仍未解决，是换电脑后继续的重点。

## 3. 重要外部/本地发现

### 3.1 xcap WGC 不能直接作为主进程修法

之前尝试过给 `xcap = "0.9.6"` 启用 `wgc` feature，希望用 WGC 类截图后端绕开 WebView2/D3D 白块。

结果：

- `cargo check` 和 `npm run build` 可以通过。
- 但真实按 `Alt+A` 后主进程会 abort。
- 日志里出现：

```text
fatal runtime error: Rust cannot catch foreign exceptions, aborting
```

结论：

- 不能直接把 `xcap` WGC feature 放进当前 Tauri 主进程里。
- 继续走这个方向需要隔离进程或单独 native helper，不能作为本轮快速修复。

### 3.2 xcap 非 WGC / legacy 截图路径仍可用

当前代码里 `capture_current_monitor_png()` 仍是：

- 优先 `xcap::Monitor::capture_image()`。
- 失败时 fallback 到 `screenshots` crate。

这个路径不会触发 WGC foreign exception，但可见 WebView2 主面板如果不先隐藏，仍可能捕到白块或残影。

### 3.3 关键根因不只是截图后端

本轮测试后确认：

- 截图前隐藏主面板并等待 DWM 合成器清空，可以让底图不再出现主面板残影。
- 但是第 4 项里主面板原本已被 `X` 隐藏，截图关闭时 Windows 焦点回落会把隐藏的 Tauri 主窗口壳激活成白屏。
- 这个白屏壳即使 `TauriWindow main visible=false`，桌面上仍能肉眼看到，说明需要更底层的焦点/owner/window style 处理。

## 4. 本轮已修改的文件

### 4.1 `tauri-client/src-tauri/src/window_lifecycle.rs`

新增/修改内容：

- 新增 `MainWindowScreenshotState`，用于记录截图开始前主面板状态：
  - `was_visible`
  - `was_minimized`
- 新增全局状态：
  - `MAIN_WINDOW_SCREENSHOT_STATE: OnceLock<Mutex<Option<MainWindowScreenshotState>>>`
- 新增 `prepare_main_window_for_screenshot(app)`：
  - 截图前记录主面板原始状态。
  - 如果主面板当前可见且非最小化，则隐藏主面板。
  - 隐藏前调用 `disable_windows_transition(&main)`，减少隐藏动画/残影。
- 新增 `wait_for_hidden_main_capture_settle()`：
  - Windows 下循环 `DwmFlush()` + `120ms` 等待，当前约 360ms。
  - 目的是等待 DWM compositor 确认隐藏后的画面稳定，再真正抓屏。
- 新增 `restore_main_window_after_screenshot(app, reason)`：
  - 如果截图前主面板可见且非最小化：截图关闭后恢复显示。
  - 如果截图前主面板最小化：保持最小化。
  - 如果截图前主面板不可见：尝试保持隐藏。
- 新增/增强 `keep_main_window_hidden_after_screenshot()`：
  - 对原本隐藏的主面板执行立即隐藏。
  - 再延迟 120ms、280ms 二次隐藏。
  - 当前事实：这仍未解决第 4 项桌面白壳。
- 增强 `robust_hide_window()`：
  - 原有 `window.hide()` + `ShowWindow(hwnd, SW_HIDE)`。
  - 新增 `SetWindowPos(... SWP_HIDEWINDOW | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE)`。
  - 新增 `InvalidateRect(0, null, true)` 和 `DwmFlush()`。
  - 当前事实：这对底图残影有帮助，但仍不能清掉第 4 项白色窗口壳。
- 修改截图窗口 `CloseRequested` / `Destroyed` 生命周期：
  - 关闭截图窗口时调用 `restore_main_window_after_screenshot(...)`。

### 4.2 `tauri-client/src-tauri/src/screenshot_commands.rs`

新增/修改内容：

- `start_screenshot_impl()` 开始时调用：
  - `prepare_main_window_for_screenshot(&app)`
- 关闭/隐藏旧截图窗口后，如果主面板被隐藏：
  - 调用 `wait_for_hidden_main_capture_settle().await`
  - 然后再进行真实桌面截图。
- 截图捕获失败时：
  - 调用 `restore_main_window_after_screenshot(&app, "capture-error")`
- 创建截图窗口失败时：
  - 调用 `restore_main_window_after_screenshot(&app, "create-screenshot-window-error")`
- `force_close_screenshots()` 和 `cancel_screenshot()` 里：
  - 调用 `restore_main_window_after_screenshot(...)`

效果：

- 可见主面板时，截图底图不再捕到主面板白块或半透明主面板。
- 截图关闭后，第 2/5 项主面板能恢复可见。

### 4.3 `tauri-client/src-tauri/src/lib.rs`

新增：

- Win32/DWM FFI：

```rust
pub fn DwmFlush() -> i32;
```

用途：

- 隐藏主面板后等待 DWM 合成器更新。
- `robust_hide_window()` 隐藏后 flush。

### 4.4 `tauri-client/src/utils/recordingWindows.ts`

新增：

- `CloseRecordingBorderWindowsOptions`
  - `source?: string`
  - `hideMain?: boolean`

修改：

- `closeRecordingBorderWindows([], options)` 调用 Rust 命令时传：
  - `source`
  - `hideMain`

原因：

- 之前普通截图 reset/cleanup 会调用录制窗口清理。
- Rust `force_close_recording_controls` 会无条件 hide main。
- 这会覆盖截图关闭后的主面板恢复，导致第 2/5 项关闭截图后主面板又被藏起来。

### 4.5 `tauri-client/src/hooks/useScreenshotRecording.ts`

修改：

- 录制完成：
  - `closeRecordingBorderWindows([], { source: "recording-finish", hideMain: true })`
- 录制取消：
  - `closeRecordingBorderWindows([], { source: "recording-cancel", hideMain: true })`
- `clearRecordingState()`：
  - 先判断是否真的有录制活动：
    - `recordingStatusRef.current !== "idle"`
    - `recordingSegmentsRef.current.length > 0`
    - `recordingPickerModeRef.current !== null`
    - `recordingStartedAtRef.current !== null`
    - `isRecordingBusyRef.current`
    - `recordingRegionRef.current !== null`
  - 只有确实有录制活动时，才清理录制边框/控制条并隐藏主面板。

效果：

- 普通截图流程不会被录制清理逻辑误伤。
- 真正录制结束/取消时仍保留隐藏主面板的保护逻辑。

## 5. 自动化桌面测试方法

我写了临时脚本：

- `.codex-analysis/desktop-smoke.ps1`

注意：

- 这是临时验证脚本，没有计划提交。
- 它运行在当前电脑桌面上，换电脑后可以按下面思路重建。

脚本做的事：

1. 停止旧 `YsnTrans.exe`。
2. 最小化其它窗口，让桌面背景更干净。
3. 用 `Start-Process -WindowStyle Hidden` 启动：
   - `tauri-client/src-tauri/target/debug/YsnTrans.exe`
4. 用 Win32 枚举当前进程窗口：
   - `EnumWindows`
   - `GetWindowTextW`
   - `GetClassNameW`
   - `IsWindowVisible`
   - `IsIconic`
   - `GetWindowRect`
5. 用 `WScript.Shell.SendKeys("%a")` 发送 `Alt+A`。
6. 等待 `%LOCALAPPDATA%/ScreenshotTranslator/fullscreen_temp.png` 更新。
7. 用 `System.Drawing.Graphics.CopyFromScreen` 保存真实桌面截图：
   - `desktop-before.png`
   - `desktop-overlay.png`
   - `desktop-after.png`
8. 发送 `Esc` 关闭截图。
9. 每个场景输出 JSON 和 summary。

早期脚本踩过的坑：

- `GetWindowTextW` / `GetClassNameW` 的 P/Invoke 一开始缺少 `CharSet.Unicode`，导致 `YsnTrans` 被读成单个字母 `Y`，第 5 项等待主窗口时卡住。
- Debug exe 默认会带黑色控制台窗口，第一次测试底图捕到了控制台窗口，干扰判断；后来用 `Start-Process -WindowStyle Hidden` 降低干扰。
- 只看 `IsWindowVisible` 不可靠，第 4 项白窗肉眼可见，但枚举中主窗口仍可能 `visible=false`。

## 6. 最新自动化测试结果

最后一轮 summary：

```text
01_hidden_alt_a_close:
  tempCreated=True
  tempLength=3068073
  alive=True
  mainBefore=visible:False,min:False,rect:182,182,1398,1021
  mainAfter=visible:False,min:False,rect:182,182,1398,1021

02_show_main_alt_a_close:
  tempCreated=True
  tempLength=3065956
  alive=True
  mainBefore=visible:True,min:False,rect:182,182,1398,1021
  mainAfter=visible:True,min:False,rect:182,182,1398,1021
  tempWhiteRatio=0.0059
  tempMean=176.08

03_show_minimize_alt_a_close:
  tempCreated=True
  tempLength=3067333
  alive=True
  mainBefore=visible:True,min:True,rect:-32000,-32000,-31840,-31972
  mainAfter=visible:True,min:True,rect:-32000,-32000,-31840,-31972

04_show_x_close_alt_a_close:
  tempCreated=True
  tempLength=1682843
  alive=True
  mainBefore=visible:False,min:False,rect:182,182,1398,1021
  mainAfter=visible:False,min:False,rect:182,182,1398,1021

05_show_taskbar_restore_alt_a_close:
  tempCreated=True
  tempLength=3067187
  alive=True
  mainBefore=visible:True,min:False,rect:182,182,1398,1021
  mainAfter=visible:True,min:False,rect:182,182,1398,1021
  tempWhiteRatio=0.0059
  tempMean=176.11
```

字段解释：

- `tempCreated=True`：截图底图文件确实生成。
- `alive=True`：截图后进程没有崩溃。
- `tempWhiteRatio`：在主面板原位置采样的纯白像素占比，约 `0.0059` 说明不是整块白窗。
- 第 2/5 项肉眼检查最新 `fullscreen_temp.png`：主面板已不在底图里，只剩真实桌面。

但是不能误判为全部通过：

- 第 4 项 `desktop-after.png` 肉眼仍能看到一个巨大白色 `YsnTrans` 主窗口壳。
- 这说明第 4 项未通过。

## 7. 当前验证命令

已通过：

```powershell
cd D:/Desktop/自制截图/tauri-client/src-tauri
cargo check
cargo build
```

```powershell
cd D:/Desktop/自制截图/tauri-client
npm run build
```

`npm run build` 只有既有 Vite warning：

- `@tauri-apps/api/window.js` 同时被静态/动态 import。
- chunk 大于 `1200 kB`。

没有发现新的 TypeScript/Rust 编译错误。

## 8. 当前未解决的关键问题

第 4 项流程：

1. 打开主面板。
2. 点击右上角 `X`，主面板按设计被隐藏。
3. `Alt+A` 开始截图。
4. 关闭截图。
5. 桌面出现一个真实白色 `YsnTrans` 主窗口壳。

日志里可见：

```text
[window-trace] source=restore-main-after-screenshot action=keep-hidden reason=force-close-screenshots was_visible=false was_minimized=false
[window-trace] source=restore-main-after-screenshot action=hide-main label=main reason=force-close-screenshots
[window-trace] source=restore-main-after-screenshot action=hide-main-delayed label=main reason=force-close-screenshots delay_ms=120
[window-trace] source=restore-main-after-screenshot action=hide-main-delayed label=main reason=force-close-screenshots delay_ms=280
```

即使这样，白窗仍出现。

这说明：

- 简单的 `window.hide()` 不够。
- `ShowWindow(hwnd, SW_HIDE)` 不够。
- `SetWindowPos(... SWP_HIDEWINDOW)` 不够。
- 延迟二次隐藏仍不够。
- 问题很可能发生在截图 overlay 关闭/隐藏时的 Windows 焦点回落或 owner 激活，而不是主窗口状态记录本身。

## 9. 换电脑后建议的下一步修复方向

### 9.1 优先方向：关闭截图 overlay 前把焦点移交给桌面/shell

假设：

- 第 4 项中，主窗口虽然隐藏，但仍是 Windows 认为可以回落激活的 owned/previous foreground window。
- 截图窗口关闭后，Windows 把焦点/激活交回隐藏主窗口，导致 Tauri/WebView2 壳以白屏形式显示。

建议实现：

- 在截图窗口关闭前，如果 `MAIN_WINDOW_SCREENSHOT_STATE.was_visible == false`：
  - 先把 screenshot overlay 设为非 topmost。
  - 先把焦点交给桌面/shell/当前前台非本应用窗口。
  - 再隐藏 screenshot overlay。
  - 再保持 main hidden。

可能需要新增 Win32 FFI：

```rust
FindWindowW("Progman", null)
FindWindowExW(...)
SetForegroundWindow(desktop_hwnd)
SetActiveWindow(0)
SetFocus(0)
```

也可以枚举所有非本进程、可见、有标题、有正常 rect 的顶层窗口，选一个作为焦点回落目标。

验收重点：

- 第 4 项 `desktop-after.png` 不能再出现白色 `YsnTrans` 壳。

### 9.2 第二方向：主窗口 X 关闭时不只 hide，而是真正 destroy/recreate

当前主窗口 CloseRequested 逻辑是隐藏：

```rust
window.hide();
ShowWindow(hwnd, SW_HIDE);
api.prevent_close();
```

这会保留同一个 Tauri/WebView2 HWND。

如果 Windows 一直能把这个 HWND 激活成白壳，可以考虑：

- `X` 关闭时真正 destroy main WebViewWindow。
- 点击托盘/任务栏/单实例唤醒时重建 main window。

风险：

- 需要确认 Tauri v2 主窗口销毁后重建是否影响 app state、tray、hotkey、invoke handler。
- 改动范围较大，但可能是最干净的根因修复。

### 9.3 第三方向：主窗口隐藏时移动到屏幕外并最小化

尝试策略：

- 对隐藏主窗口同时执行：
  - `set_skip_taskbar(true)`，如当前设计允许。
  - `minimize()`。
  - `set_position(-32000, -32000)` 或使用 Windows 原生 offscreen。
  - `ShowWindow(SW_HIDE)`。

缺点：

- 容易产生状态恢复复杂度。
- 第 2/5 项需要恢复原始位置。
- 商业级上不如焦点/owner 根因处理干净。

### 9.4 第四方向：截图 overlay 关闭时不要 close，只隐藏并复用

当前截图关闭会 `force_close_screenshots()`，对 primary screenshot window 是 hide，对 secondary 可能 close。

仍需确认：

- 前端 `Esc` 路径到底调用了 `force_close_screenshots`，还是触发了 window CloseRequested。
- 如果 overlay 关闭动作导致焦点回落，可以避免真正关闭，只 hide overlay 并手动取消激活。

这方向要结合日志继续查。

## 10. 继续测试时必须保留的标准

不要只看 JSON 字段，必须同时看图：

- `fullscreen_temp.png`：程序真正用作截图底图的图片。
- `desktop-before.png`：操作前真实桌面。
- `desktop-overlay.png`：截图 overlay 显示时真实桌面。
- `desktop-after.png`：关闭截图后真实桌面。

尤其第 4 项：

- 即使 `mainAfter.visible=false`，也必须人工看 `desktop-after.png`。
- 如果图片中还有大白色 `YsnTrans` 窗口，仍然失败。

## 11. 推荐换电脑后的自动化测试脚本重建要点

PowerShell 需要：

- `Add-Type -AssemblyName System.Windows.Forms`
- `Add-Type -AssemblyName System.Drawing`
- Win32 P/Invoke：
  - `EnumWindows`
  - `GetWindowThreadProcessId`
  - `IsWindowVisible`
  - `IsIconic`
  - `GetWindowTextW` with `CharSet.Unicode`
  - `GetClassNameW` with `CharSet.Unicode`
  - `GetWindowRect`
  - `ShowWindow`
  - `SetForegroundWindow`
  - `SetCursorPos`
  - `mouse_event`

启动方式：

```powershell
Start-Process -FilePath "tauri-client/src-tauri/target/debug/YsnTrans.exe" `
  -WorkingDirectory "tauri-client/src-tauri/target/debug" `
  -WindowStyle Hidden `
  -RedirectStandardOutput ysntrans-out.log `
  -RedirectStandardError ysntrans-err.log `
  -PassThru
```

重要：

- 如果使用 debug exe，它可能带 console；必须用 `-WindowStyle Hidden`。
- 先启动 Vite dev server 或确保 devUrl 可用：
  - `http://127.0.0.1:1420`
- 或直接测 release/bundled exe，避免 dev server 干扰。

## 12. 当前工作区注意事项

本轮生成过临时目录：

- `.codex-analysis/`

里面有：

- 抽帧图片。
- 多轮 smoke 图片。
- 临时 PowerShell 自动化脚本。

这个目录是临时证据，不应提交到 Git。

换电脑后，如果没有这个目录，不影响继续；按本文档重建即可。

## 13. 当前代码状态的真实评价

可以保留的进展：

- 可见主面板时，截图前隐藏 + DWM settle 能清掉底图残影。
- 截图关闭后，第 2/5 项主面板恢复状态正确。
- 最小化路径保持最小化。
- 普通截图 reset 不再被录制清理误伤主面板恢复。
- 构建门禁通过。

不能宣称完成：

- 第 4 项仍失败。
- `TauriWindow visible=false` 与肉眼可见白窗冲突，说明还缺少真实桌面层面的窗口/focus 处理。

下一轮第一任务：

- 专门修第 4 项：`X` 隐藏主面板后，截图关闭不能在桌面留下白色 `YsnTrans` 壳。

