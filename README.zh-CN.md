# YsnTrans 截图翻译器

**YsnTrans 截图翻译器** 是一款面向 Windows 的桌面效率工具，提供快速截图、OCR、翻译、标注、贴图、滚动截图和录屏能力。它适合需要快速捕捉信息、在原图语境中理解文字、并保留可视化笔记的用户。

> 默认文档为英文。English documentation: [README.md](README.md).

## 发布状态

- 当前发布目标：**v1.2.7**
- 支持平台：**Windows**
- 应用名称：**YsnTrans**
- 核心技术栈：**Tauri 2 + React + TypeScript + Rust**
- OCR 方向：内置、产品自有的 **RapidOCR / ONNXRuntime** 本地优先流程

## 核心亮点

- **快速截图流程**：支持托盘操作和全局快捷键 `Alt+A` 进入截图。
- **截图翻译**：选择区域后执行 OCR 和翻译，并把译文重绘到原图区域。
- **本地 OCR 管线**：围绕内置本地 OCR 资产设计，核心截图识别流程不依赖云端 OCR。
- **智能窗口识别**：结合 Windows 窗口边界、DWM 边框、显示器工作区和系统窗口过滤，改善浏览器边缘、任务栏附近、显示器边界的识别体验。
- **标注工具**：支持矩形、椭圆、箭头、画笔、马赛克、文字、撤销和重做。
- **贴图**：可将选中区域置顶悬浮，便于对照和记录。
- **滚动截图**：选择区域后采集并拼接长页面或可滚动窗口内容。
- **录屏**：支持区域录制、帧率、分辨率、麦克风、系统声音和 FFmpeg 配置。
- **结果窗口**：OCR 和翻译结果可在独立窗口查看与复制。
- **可靠恢复路径**：关键流程提供取消、强制关闭、本地回退和状态反馈。

## 下载与安装

当前已整理并验证的便携发布包：

```text
build/x64_v1.2.7/ScreenshotTranslator_Windows.zip
```

本地开发和 smoke 测试使用的便携目录：

```text
release/YSN-Screenshot-Translator/YsnTrans.exe
```

完整安装包由 `build.bat` 或 `npm run tauri build` 生成，位置在 `tauri-client/src-tauri/target/release/bundle/`。如果 Windows SmartScreen 弹出提示，请按未签名桌面软件的标准确认流程继续安装。面向更大范围分发前建议补充代码签名。

## 快速开始

1. 安装并启动 **YsnTrans**。
2. 按 `Alt+A` 进入截图模式。
3. 拖拽选择区域，然后在工具栏中选择复制、保存、贴图、OCR、翻译、标注或录屏。
4. 随时按 `Esc` 退出截图状态。

## 快捷键

| 快捷键 | 功能 |
| --- | --- |
| `Alt+A` | 启动截图 / 二次截图 |
| `Alt+T` | 启动截图翻译 |
| `Esc` | 退出当前截图状态 |
| `Alt+F4` | 强制关闭截图覆盖层 |
| `Ctrl+C` | 复制当前截图结果 |
| `Ctrl+S` | 保存当前截图结果 |
| `Ctrl+Q` | 翻译当前选区 |
| `1` | 矩形工具 |
| `2` | 椭圆工具 |
| `3` | 箭头工具 |
| `4` | 画笔工具 |
| `5` / `T` | 文字工具 |
| `6` | 马赛克工具 |
| `P` | 将当前选区贴图 |
| `Ctrl+Z` | 撤销标注 |
| `Ctrl+Y` / `Ctrl+Shift+Z` | 重做标注 |

## 功能说明

### 截图与窗口捕捉

YsnTrans 使用专用截图覆盖层窗口和本地图片缓存来降低启动延迟。窗口目标检测结合 Windows 窗口边界、DWM 扩展边框、显示器工作区和系统窗口过滤，提升浏览器边缘、显示器边界和 Windows 任务栏附近的选择准确性。

### OCR 与翻译

产品长期方向是内置 RapidOCR / ONNXRuntime 管线，并配套 PP-OCR 模型资产。应用设计目标是自动处理源语言，目标语言由用户选择，默认目标语言为简体中文。

### 录屏

录屏依赖 FFmpeg。应用可使用软件同级目录下的 `ffmpeg/ffmpeg.exe`，也可使用用户手动选择的 FFmpeg 路径，或按配置下载 FFmpeg 运行时。

### 滚动截图

选择区域后可启动滚动截图，用于网页、文档视图等长内容采集。为获得更稳定结果，采集过程中请保持目标窗口可见，并避免改变缩放或布局。

## 仓库结构

```text
.
├─ build.bat                         # Windows 便捷构建脚本
├─ docs/                             # 产品方向与实现历史
├─ ffmpeg/                           # 可选 FFmpeg 运行时位置
├─ models/                           # 打包使用的 OCR 模型资产
├─ release/                          # 本地组装发布产物
├─ scripts/                          # 构建与发布辅助脚本
├─ server/                           # 翻译服务组件
└─ tauri-client/
   ├─ src/                           # React 前端
   │  ├─ components/                 # UI 组件
   │  ├─ hooks/                      # 截图、OCR、录屏和 UI Hooks
   │  ├─ pages/                      # 应用页面
   │  ├─ types/                      # 共享 TypeScript 类型
   │  └─ utils/                      # 前端工具
   └─ src-tauri/                     # Tauri/Rust 后端
      ├─ resources/                  # 内置运行时资源
      ├─ icons/                      # 应用图标
      └─ src/                        # 命令、快捷键、OCR、录屏和窗口逻辑
```

## 开发

环境要求：

- Windows
- Node.js 和 npm
- Rust stable 工具链
- Tauri 2 Windows 开发依赖

启动开发版：

```bash
cd tauri-client
npm install
npm run tauri dev
```

运行检查：

```bash
cd tauri-client
npm run build
cd src-tauri
cargo check
```

## 构建发布版

发布前更新版本号：

- `tauri-client/package.json`
- `tauri-client/package-lock.json`
- `tauri-client/src-tauri/Cargo.toml`
- `tauri-client/src-tauri/tauri.conf.json`

构建便携目录和安装包：

```bash
build.bat --no-pause --no-launch
```

生成或刷新便携 zip：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1
```

生成产物位于：

```text
release/YSN-Screenshot-Translator/
build/x64_v<version>/
tauri-client/src-tauri/target/release/bundle/
```

## 质量标准

YsnTrans 按商业级桌面效率产品维护。发布变更应重点验证：

- 截图启动延迟和覆盖层恢复能力
- OCR 正确性和回退路径
- 翻译质量和技术标识符保留
- 录屏开始/停止可靠性
- FFmpeg 与模型资产打包
- Windows 任务栏、DPI、多显示器和浏览器边缘行为
- 构建产物清晰、安装包可复现

## 分发注意事项

- 大体积 OCR 模型和运行时文件不应随意提交，除非发布计划明确要求。
- 未签名安装包可能触发 Windows SmartScreen 提示。
- FFmpeg 可继续作为外部运行时，除非录屏未来成为产品自有战略运行时。
- 长期产品方向以 `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 为准，实现历史记录在 `docs/IMPLEMENTATION_CHAPTERS.md`。

## 许可与归属

本仓库由 YsnTrans 发布者维护。面向广泛公开分发前，应补充最终许可证、代码签名、支持渠道和隐私说明。
