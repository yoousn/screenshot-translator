# YSN 客户端原生 Rust 截图方案设计规范

本规范定义了 YSN 截图翻译系统 Tauri 客户端的纯 Rust + React 原生截图实现方式，彻底剥离并废弃对旧 Qt C++ 可执行程序 `ScreenshotTranslator.exe` 的调用。

---

## 1. 架构概览

新架构不再启动任何外部程序，而是使用 Rust 的生态库进行屏幕捕获与二进制处理，具体数据流向如下：
1. **主窗口 (Dashboard)** 点击“立即截图” -> 触发 Rust 命令 `start_screenshot`。
2. **Rust 层** 使用 `screenshots` 库获取主显示器（或鼠标所在显示器）的当前整张屏幕图像，将其保存为 AppData 临时目录中的 PNG 图片，并移动、显示预设好的 `screenshot` 辅助窗口。
3. **Tauri 截图窗口 (Screenshot Window)** 唤起为全屏、无边框、置顶。前端 React Canvas 加载该全屏图片作为背景，遮罩半透明覆盖。
4. **Canvas 交互**：
   - 鼠标拖拽框选任意矩形区域。
   - 实时绘制选区，并展示选区尺寸（如 `640 x 480`）。
   - 按下 `Esc` 或点击右键取消截图，关闭窗口。
   - 双击选区或按下 `Enter` 键确认选区，将裁剪数据传给 Rust 命令进行交付，并关闭截图窗口。
5. **Rust 层裁剪与交付**：
   - `capture_region`：接受裁剪区域参数，读取原全屏图片，使用 `image` 库裁剪出子图像并返回 Base64 字符串。
   - `copy_image_to_clipboard`：接受 Base64 图片，使用 `arboard` 库将该图片写入 Windows 系统剪贴板。
   - `save_image_to_file`：接受 Base64 图片，弹出系统保存对话框将其保存为本地 PNG 文件。

---

## 2. 窗口配置与 Rust 库依赖

### 2.1 Tauri 窗口配置 (`tauri.conf.json`)
新增一个 label 为 `screenshot` 的辅助窗口：
- `"decorations": false` (无系统标题栏和边框)
- `"transparent": true` (透明背景)
- `"alwaysOnTop": true` (最前端置顶)
- `"visible": false` (初始化隐藏)
- `"skipTaskbar": true` (任务栏隐藏)
- `"fullscreen": false` (由 Rust 动态调节其尺寸覆盖整个屏幕，以确保跨高 DPI 屏幕完美渲染)

### 2.2 Cargo 依赖 (`Cargo.toml`)
引入以下核心库：
- `screenshots = "0.8"`：跨平台屏幕截图捕获。
- `arboard = "3.4"`：跨平台剪贴板控制，写入裁剪后的图像。
- `image = { version = "0.25", default-features = false, features = ["png"] }`：内存裁剪图片并保存为 PNG 字节流。
- `base64 = "0.22"`：处理 Base64 编解码。

---

## 3. 功能开发与清理硬限制

1. **废弃旧 exe 路径**：
   - 移除 `start_screenshot` 中对 `release\ScreenshotTranslator.exe` 的调用。
   - 彻底在主工程中禁用/删除所有与 `ScreenshotTranslator` 或旧 release 目录的字符串及命令行交互。
2. **第一阶段功能聚焦**：
   - 本次仅实现截图、裁剪、复制剪切板与本地保存功能。
   - 不进行矩形框、画笔线条等第二阶段标注图层的渲染。
3. **真实错误透传**：
   - 截图或剪裁过程中如遇错误（如没有屏幕设备、内存溢出等），必须把真实 error 作为 Result 的 Err 返回给前端，并由前端 `message.error` 进行醒目报错，绝不隐瞒。
