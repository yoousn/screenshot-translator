# 商业级改造执行记录

> 本文档是唯一施工日志，但不再逐章保留超长全文。旧的 1–97 章已压缩为里程碑摘要；后续只记录“最近章节详情 + 当前交接状态”。主方向以 `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 为准。

## 当前交接状态（2026-06-05）

## Chapter 157：录制工具栏实时跟随、窗口清理门禁与安全审查

### 目标

- 修复截图选区拖拽/缩放时，包含录制按钮的截图工具栏不随选区实时移动的问题。
- 修复 `Alt+A` 初始化截图时清理不存在的录制窗口可能唤醒空白 `recording_notice` 窗口的问题。
- 按用户要求做一轮源码级全量审查，确认是否仍存在超过 800 行且需要拆分的源文件，并修复审查中发现的真实安全/兼容 BUG。

### 实际完成

- `ScreenshotPage.tsx` 增加基于 `requestAnimationFrame` 的工具栏 DOM 位置同步器：
  - 复用 `getActionToolbarStyle` 计算位置。
  - 在拖拽、缩放和实时绘制选区时直接写入 toolbar DOM 的 `top/left`，绕开 React 状态重绘延迟。
  - 页面卸载时清理未完成 RAF，避免截图窗口关闭后的延迟 DOM 写入。
- `useScreenshotInteraction.ts` 增加 `syncToolbarPosition` 回调，并把所有实时 rect 更新路径统一接入。
- `recordingWindows.ts` 重写窗口关闭前的存在性门禁：
  - 新增 `getWindowByLabelIfExists`，先用 `WebviewWindow.getAll()` 枚举真实存活窗口。
  - `closeWindowIfExists` 与 `waitForWindowGone` 不再调用 `WebviewWindow.getByLabel()`。
  - 不存在的 `recording_notice` / `recording_overlay` 会直接跳过，避免意外唤醒空白窗口。
- `useRecordingControl.ts` 复用安全 `closeWindowIfExists`，移除本地 `getByLabel` 清理逻辑；录屏排除改为使用当前动态控制条 label。
- 安全审查修复：
  - `server/app.py` 不再明文打印 `client_token`，认证失败不再输出 received/expected token。
  - 翻译器缓存 key 不再保存 API key 前缀，改为不可逆 SHA256 指纹。
  - `server/config.py` 首次生成配置时日志只打印 redacted token，返回值仍保留真实 token。
  - `localOcrTranslate.ts` 请求日志不再打印翻译服务 token。
  - `server/security.py` 修复 DeepL 官方域名在代理 DNS 下解析到 `198.18.*` 时被误判为私网 SSRF 的问题，同时保持非白名单公网 URL 解析到私网时继续拒绝。
- 新增 `server/tests/test_security.py` 覆盖 DeepL 官方域名代理 DNS 豁免与普通私网 DNS 拒绝。
- 源码行数复查：
  - 源码/脚本范围内没有超过 800 行的 `.ts/.tsx/.rs/.py/.ps1/.mjs/.js` 文件。
  - 当前最大源码文件为 `useScreenshotInteraction.ts` 744 行、`ScreenshotPage.tsx` 716 行、`rapidocr_runner.py` 715 行，均低于 800 行。
  - 超过 800 行的仅有 `package-lock.json`、`docs/IMPLEMENTATION_CHAPTERS.md`、OCR 模型/runner 二进制和 release 产物，不按业务源码模块拆分。

### 新增文件

- `server/tests/test_security.py`

### 修改文件

- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
- `tauri-client/src/utils/recordingWindows.ts`
- `tauri-client/src/hooks/useRecordingControl.ts`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `server/app.py`
- `server/config.py`
- `server/security.py`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不提交、不推送、不打 tag。
- 不拆分锁文件、模型文件、图标、runner 二进制或 release 产物。
- 不改变 OCR 主线、FFmpeg 参数、翻译通道业务语义或用户本机配置。

### 验证

- `npm run check:i18n`：通过，566 keys match。
- `npm run check:ocr-processing`：通过。
- `npm run build`：通过，仍只有既有 Vite 动态导入/chunk size warning。
- `cargo test`：通过，19 passed。
- `python -m py_compile server\app.py server\config.py server\security.py server\translator.py`：通过。
- `python -m pytest server\tests`：通过，33 passed / 1 skipped。
- `git diff --check`：通过，仅有工作区 LF 将被 Git 转 CRLF 的提示。

### 当前风险

- 已通过构建、Rust 测试、Python 测试和静态门禁；真实 `Alt+A -> 框选 -> 拖动/缩放选区 -> 工具栏跟随` 与 `Alt+A` 不弹空白录制窗口仍建议在 Windows 桌面人工 smoke。

### 下一章建议

- 继续做真实 Windows 录制流程 smoke：连续三次 `Alt+A -> 框选 -> 录制准备 -> 关闭`，并验证开始、暂停、继续、停止保存、取消清理后没有白屏窗口或残留控制条。

## Chapter 151：前端巨石代码拆解：截图批注 Hooks 抽取

### 目标

继续响应用户指令，拆分 `ScreenshotPage.tsx` 中的巨石逻辑。本轮聚焦于独立性最强的“标注管理模块”，将其剥离出主文件，进一步减轻 React UI 组件的心智负担，同时保障编译链完全不受损。

### 本章实际处理

- **抽取 `useScreenshotAnnotation` Hook**：
  - 将庞大的批注状态管理（包含 10 个以上 `useState` 与相关的 `useRef`，如 `annotationTool`, `annotationColor`, `annotations`, `annotationHistory`, `draftAnnotation` 等）和相关的核心操作函数（如 `pushAnnotationHistory`, `undoAnnotation`, `redoAnnotation`, `commitTextDraft`, `deleteSelectedAnnotation`）从 `ScreenshotPage.tsx` 提取到了纯逻辑文件 `tauri-client/src/hooks/useScreenshotAnnotation.ts`。
  - 保留了与 Canvas 重绘生命周期的交互，通过向 Hook 传入 `onRenderNeeded` 回调来无缝触发重绘。
  - 将与批注默认属性相关的硬编码常量（如 `DEFAULT_ANNOTATION_COLOR`, `DEFAULT_ANNOTATION_TOOL`）也一并集中封装在 Hook 文件顶部导出。

- **`ScreenshotPage.tsx` 安全更新**：
  - 成功移除了超过百行的状态定义、引用绑定以及增删改查实现函数，将其收敛至一行 Hook 的调用。
  - 通过 `replace_file_content` 和脚本自动化精确移除了旧版的所有硬编码操作，修复了因为抽取造成的 TS 类型断层，所有旧的 refs 已平滑迁移至 Hook 管理的闭包中。

### 修改文件

- `tauri-client/src/hooks/useScreenshotAnnotation.ts` (新增)
- `tauri-client/src/pages/ScreenshotPage.tsx`

### 验证

- `cargo check`：通过。
- `npm run check:i18n`：通过，564 keys match。
- `npm run build`：通过。无 TS 类型缺失与语法错误。

### 下一步建议

Chapter 152：`ScreenshotPage.tsx` 中仍然含有深耦合的画图逻辑与 OCR/翻译逻辑。下一步建议提取 `useScreenshotOcr.ts`（处理翻译与 OCR 发起及结果预览窗口状态）和 `useScrollCapture.ts`（滚动截图机制），以完成对主要纯业务状态流转的全面剥离，最终将 `ScreenshotPage.tsx` 还原为存粹的事件绑定与子组件容器。


## Chapter 150：主窗口幽灵问题闭环与前端模块化/字典拆分

### 目标

彻底解决录制条关闭后出现的 `YsnTrans` 白色幽灵窗口问题，实现项目的窗口生命周期边界重构；并在确保 `npm run build` 和 `cargo check` 通过的前提下，推进前端 React 巨石逻辑抽取和硬编码字典拆分。

### 本章实际处理

- **主窗口生命周期闭环修复：**
  - 合并 `window_control.rs` 到 `window_lifecycle.rs`，确立唯一窗口控制源。
  - 引入了 `robust_hide_window` 机制：直接调用 Windows 原生 `win32::ShowWindow(hwnd, 0)`。这绕过了 Tauri 内部 `is_visible` 状态缓存对操作系统焦点事件的误判，从根本上杀死了关闭录制条时导致的 YsnTrans 幽灵窗口复现。
  - 清理了 `lib.rs` 中的冗余引用。

- **前端巨石模块拆分：**
  - 提取了 `RecordingControlPage.tsx` 中的厚重状态机与事件绑定，创建了纯逻辑的 `useRecordingControl.ts` 钩子。
  - 页面组件现已完全隔离 UI 渲染和状态流转逻辑。

- **多语言 (i18n) 字典拆分：**
  - 将 `ScreenshotToolbar.tsx` 中零散的中文硬编码提示词（如“矩形标注”、“画笔”、“截图翻译”等）提取到 `tauri-client/src/i18n/dictionaries.ts` 的 `toolbar` 命名空间。
  - 更新了 `types.ts` 定义，并通过 `useI18n` Hook 将这些静态字符串动态化，支持英文兜底。

### 修改文件

- `src-tauri/src/lib.rs`
- `src-tauri/src/window_lifecycle.rs`
- `tauri-client/src/hooks/useRecordingControl.ts` (新增)
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/components/screenshot/ScreenshotToolbar.tsx`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/i18n/types.ts`

### 验证

- `cargo check`：通过。
- `npm run check:i18n`：通过，564 keys match。
- `npm run build`：通过。
- 幽灵白窗复现条件消失：已验证 `win32::ShowWindow` 兜底控制。

### 下一步建议

Chapter 151：继续推进前端 `ScreenshotPage.tsx` (2500+ 行) 剩余部分的拆分。建议从不直接绑定 `Canvas` 上下文的纯逻辑配置区（如 OCR 识别模式切换、配置菜单等）开始，逐步用更细粒度的 Hooks 取代巨石结构。



### 当前阶段

- 当前主线：产品内置 `RapidOCR / ONNXRuntime` OCR 主路径。
- 当前最新完整验证章节：Chapter 148。
- 当前正在推进章节：Chapter 149：继续做真实覆盖层肉眼 smoke、动态 small-text retry 与 OCR worker 细分打点。
- Chapter 148 当前状态：已完成，服务端翻译批处理超时预算、生产缓存测试、LLM 慢失败防护、翻译覆盖字号保险和截图状态机误触防护均已通过本地验证。
- 关键原则：旧自研 `YSN OCR Runtime` 已废弃为非主路径；普通主流程只走 RapidOCR runner。OCR ready 必须由打包 runner、自测、fixture、真实 `Ctrl+D` 结果窗和翻译覆盖层共同证明。

### 当前已验证命令

- Chapter 147 验证已通过：
  - `npm run check:ocr-processing`
  - `npm run build`
  - `npm run check:ocr-fixtures`
  - `cargo check`
  - `cmd /c "build.bat --no-pause"`
  - `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`
  - release smoke：新版 `release\YSN-Screenshot-Translator\tauri-client.exe` 启动后保持存活。
  - 侧栏 UIA 串台门禁：小选区只保留真实子文本，整列聚合文本回落 RapidOCR。
  - 透明图标像素检查：`app.ico` / `icon.png` / `taskbar-32x32.png` 不再有不透明白底像素。

### 当前未完成事项

- Chapter 149 下一步：继续优化真实用户感知延迟，优先做动态 small-text retry，并把 OCR worker warm/cold、detector、recognizer、文本源拒绝原因和翻译服务耗时写入可复制诊断报告。
- 打包版 RapidOCR runner 已改为 Python 3.12 onedir 便携产物；迁移电脑时复制整个 `release\YSN-Screenshot-Translator` 目录，不要只复制 exe。
- Windows 主窗口前台 `Alt+A` 首击框选已做自动 smoke，但仍建议用户回来后在自己的多屏/DPI/杀毒环境下再做人工复测。
- 仍需用户用侧边栏样例做一次肉眼复测，确认 UIA 保守过滤后不再出现单词被整列父容器污染。

### 当前工作树提醒

- 当前工作树包含本章未提交改动和新增文件。
- 不要随意 reset / clean。
- 不要 commit / push / tag，除非用户再次明确要求。
- 换电脑前如果需要同步，建议先由用户决定是否提交当前工作树或打包整个目录。

## 压缩里程碑摘要（Chapter 1–97）

### Chapter 1–10：主窗口、配置中心、OCR 结果与录制基础

- 建立主窗口中英双语基础和商业级 Dashboard 骨架。
- 将“模型/视频配置”重构为“识字模型 / 视频录制”配置中心。
- 拆分配置页、Dashboard、设置页等大文件，开始遵守单组件/单职责文件规则。
- OCR 结果窗口按钮改为“复制并关闭”。
- 截图态补充 `Ctrl+D` OCR 快捷键。
- 录制流程开始转向 Snow Shot 风格：准备态、控制条、状态色、自动保存方向。

### Chapter 11–25：录制闭环、诊断与商业检查入口

- Snow Shot 风格录制控制条逐步成型：准备态蓝框、录制红框、暂停黄框、底部胶囊控制条。
- 默认保存目录切到系统视频目录下 `YSN`。
- 补齐打开目录、复制视频、取消清理、临时片段等基础能力。
- 建立诊断报告、readiness / recovery 展示和 `check_commercial.ps1` 商业检查入口。
- 前端 i18n 检查脚本、OCR processing 检查脚本接入门禁。

### Chapter 26–45：OCR/翻译处理链路与 UI 质量门禁

- OCR 后处理开始覆盖图标误识别过滤、英文空格恢复、虚拟行合并、translation payload 对齐。
- 翻译 prompt 与重绘策略开始围绕短 UI 文案、技术词保护、行数对齐和减少灰块改造。
- 配置中心、Dashboard、设置页进一步拆分为卡片、hook、service、utility。
- 建立更多前端/后端测试，减少乱码文案和重复逻辑。

### Chapter 46–70：YSN OCR Runtime 架构主线

- 明确 OCR 是战略能力，主线不再是外部 PaddleOCR-json `.exe`，而是自有 ONNX Runtime + managed model packs。
- 建立 model index / manifest schema、自动源语言策略、目标语言默认简体中文、多语言 baseline。
- 建立 source readiness、model pack readiness、diagnostics readiness、runtime readiness steps。
- PaddleOCR-json 降级为兼容模式，普通用户主线转向 `YSN OCR Runtime`。

### Chapter 71–83：模型源、路由、decode/postprocess 与 crop plan

- 固化 managed source publish layout、source index dry-run、配置中心 dry-run UI。
- 建立模型源四阶段恢复说明和 source index import/dry-run 基础。
- 建立 decode/postprocess 模块入口、ONNX f32 输出摘要、decode pipeline plan。
- 建立 recognition 文本行 crop plan，为 detector → recognizer 链路准备输入。

### Chapter 84–92：recognition preprocess、CTC bridge、dictionary artifact

- crop image bytes 接入 recognition preprocess adapter。
- 建立 ONNX input binding blocker plan。
- 建立 recognition logits → CTC decode bridge。
- 建立 recognition dictionary artifact contract、字典 loader、SHA256/size 验证。
- model pack download plan 从 model-only 扩展为 model + dictionary artifact。
- active health 从 model-only 扩展为 model/dictionary artifact。
- 配置中心展示 Active OCR artifacts，并区分 Model / Dictionary。

### Chapter 93：长期文档体系收敛

- 长期文档收敛为两份：
  - `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
  - `docs/IMPLEMENTATION_CHAPTERS.md`
- 根目录 `AGENTS.md` 增加无人连续执行规则。
- 明确不再保留五六份分散计划。

### Chapter 94：Managed Source Index 支持字典 Artifact 元数据

- source index 从 model-only 扩展为 model / dictionary artifact 级元数据。
- 新增 `artifacts` 数组，旧 `models` 兼容为 `artifactType: model`。
- dictionary artifact 可写入 `contract.dictionary.source`、`sha256`、`size`。
- template、dry-run、import、测试都覆盖 dictionary artifact。
- 完整商业检查通过。

### Chapter 95：本地 Managed Source Fixture 与 Artifact 激活 Smoke

- 抽出 active artifact safe path / verify / activate 纯函数。
- 用本地 fixture 证明 dry-run download plan → SHA 校验 → active artifact 激活闭环。
- 覆盖 model artifact 与 dictionary artifact。
- 完整商业检查通过。

### Chapter 96：下载器 Artifact Size 校验与失败恢复

- `OcrArtifactDownloadPlan` 增加 expected size。
- `parse_artifact_download_plan` 要求 `size > 0`。
- `verify_and_activate_downloaded_artifact` 激活前校验文件大小，失败时删除临时下载文件。
- active artifact health 展示 expected/actual size，并报告 size mismatch。
- 完整商业检查通过。

### Chapter 97：Active Artifact Health 损坏状态进入 Readiness Blocker

- `active_model_missing` 改为基于 active health 的 `ok=false` 判断 blocker。
- size mismatch、SHA mismatch、缺失文件都能进入 blocker id 列表。
- self-test 前置检查不再只看文件是否存在。
- 完整商业检查通过。

## Chapter 98：ONNX Runtime 真实 Session 自测入口

### 目标

先建立结构化的 ONNX session readiness probe，不直接打开 runtime ready：

- 缺失模型返回结构化 blocker。
- 损坏/非 ONNX 文件返回结构化 blocker。
- session metadata 成功时返回 inputs / outputs 元数据。
- 即使 session metadata 成功，`runtimeInferenceReady` 仍保持 `false`。

### 已修改文件

- `tauri-client/src-tauri/src/ysn_ocr_runtime_adapter.rs`
  - 新增 `probe_onnx_session_readiness`。
  - 对缺失模型、非文件路径、session load failed 返回结构化状态。
  - 成功加载 metadata 时返回 `session-metadata-ready`，但 `runtimeInferenceReady=false`。
  - 新增缺失模型与损坏模型测试。
- `tauri-client/src-tauri/src/ysn_ocr_runtime.rs`
  - 新增 Tauri 命令 `probe_ysn_ocr_model_session_readiness_by_id`。
- `tauri-client/src-tauri/src/lib.rs`
  - 注册 `probe_ysn_ocr_model_session_readiness_by_id` 命令。

### 验证

- `cargo fmt`：通过。
- `cargo test ysn_ocr_runtime_adapter --lib`：通过，`11 passed; 0 failed`。
- `cargo test ysn_ocr_runtime --lib`：通过，`13 passed; 0 failed`。
- `cargo check`：通过。
- 已在仓库根目录执行 `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`：通过。
- i18n 检查结果：通过，`446 zh-CN keys match 446 en-US keys`。
- OCR processing integrity：通过。
- 前端生产构建结果：通过。
- Rust check 结果：通过。
- Rust 测试结果：通过，`98 passed; 0 failed`。

### 下一步建议

Chapter 99：真实 ONNX inference probe 与 decode/postprocess 接线。

建议处理：

- 在不改变 `runtimeInferenceReady=false` 的前提下，继续接真实 ONNX inference probe。
- 将真实输出摘要接入 decode pipeline plan。
- 对缺失模型、损坏模型、输入形状不匹配、输出类型不匹配保持结构化 blocker。

## Chapter 101：PP-OCRv5 ONNX 模型项目根目录安装

### 目标

把 PP-OCRv5 基础模型从“未来配置目标”推进到“本机已有可加载文件”的状态，为下一步截图 OCR 主流程接入 YSN Runtime 做准备。

### 新增文件

- `scripts/install_ppocrv5_onnx_models.ps1`
  - 下载官方 PaddleOCR PP-OCRv5 原始 inference 包到 app data source 目录。
  - 下载已转换 ONNX 验证模型与字典到 app data active 目录。
  - 生成 `installed-artifacts.json`。
  - 如果 `manifest.json` 已存在，同步模型和字典的 SHA256、size、source、license 与 installed 状态。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
  - 当前优先级从配置页收敛改为截图 OCR / 翻译主流程可验证闭环。
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - 记录 Chapter 101 模型下载安装到项目根目录 active 目录的事实和下一章入口。

### 项目根目录已安装目录

- Source 原始模型：`models/ocr/source/paddleocr-v5`
- Active ONNX 模型：`models/ocr/active/models`
- Active 字典：`models/ocr/active/dictionaries`
- Manifest：`models/ocr/manifest.json`

### 已安装模型

- `det-default.onnx`：PP-OCRv5 detection ONNX。
- `cls-default.onnx`：textline orientation ONNX。
- `rec-cjk.onnx` + `cjk.txt`：中文 / CJK 识别。
- `rec-latin.onnx` + `latin.txt`：拉丁字母识别。
- `rec-korean.onnx` + `korean.txt`：韩文识别。
- `rec-cyrillic.onnx` + `cyrillic.txt`：西里尔 / 斯拉夫文字识别。
- `rec-arabic.onnx` + `arabic.txt`：阿拉伯文字识别。
- `rec-thai.onnx` + `thai.txt`：泰文识别。

### 本章不做

- 不把 `runtimeInferenceReady` 改成 `true`。
- 不声称已经完成 detector → crop → recognizer → OCR blocks 的端到端生产 OCR。
- 不继续扩展配置页 JSON/source index/Probe 面板。
- 不删除 `PaddleOCR-json` 兼容路径，但下一章必须把它降级为兜底。

### 验证

- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install_ppocrv5_onnx_models.ps1`：通过，可重复执行。
- Manifest/file SHA 校验脚本：通过，`pack status: installed`，`checked models: 8`，`errors: []`。
- Python ONNX Runtime session 加载：通过，8 个 `.onnx` 均可创建 `CPUExecutionProvider` session。

### 下一章建议

Chapter 102：修 `Ctrl+D` 白屏并把截图 OCR 主流程切到 `YSN OCR Runtime / ONNX` 优先。

1. 结果窗必须在 OCR 空结果、错误、payload 超时、模型缺失时显示产品级兜底内容。
2. 后端新增 YSN OCR 主流程入口：先用 active manifest 加载 detector/recognizer，失败再进入兼容 OCR。
3. 不完成真实 OCR blocks 输出前，配置页只能显示“模型已安装 / 端到端待验证”，不能显示生产 ready。
## Chapter 102：截图 OCR 主流程接入 YSN ONNX

### 目标

把 `Ctrl+D` 截图 OCR 从“只检查模型 / 不再白屏”推进到“优先使用项目根目录 PP-OCRv5 ONNX 模型跑真实 OCR blocks”。

### 修改文件

- `tauri-client/src-tauri/src/ysn_ocr_runtime_adapter.rs`
  - 新增完整 f32 输出结构和 `run_onnx_nchw_f32_outputs`，让后端能拿到 detector probability map 与 recognizer CTC logits。
- `tauri-client/src-tauri/src/lib.rs`
  - `run_local_ocr_sync` 改为优先走 YSN OCR Runtime，不再静默回退 `PaddleOCR-json`。
  - 新增最小端到端链路：读取 active manifest → detector ONNX → DB probability decode → crop → CJK recognizer ONNX → dictionary CTC decode → `OcrBlock`。
  - detector 没检出文本框时，用整张截图作为兜底识别区域，避免直接空结果。
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - OCR 失败也打开结果窗，显示截图预览、错误原因和下一步。
  - OCR 空文本时显示产品级提示，不再白屏。
- `tauri-client/src/pages/OcrPage.tsx`
  - payload 未到前显示“正在加载 OCR 结果...”，避免纯白窗口。
- `tauri-client/src-tauri/src/ysn_ocr_runtime.rs`
  - 模型根目录优先解析项目根目录 `models/ocr`，支持 `YSN_OCR_MODEL_ROOT` 覆盖。
- `scripts/install_ppocrv5_onnx_models.ps1`
  - 默认安装到项目根目录 `models/ocr`，不再使用 C 盘 appdata。

### 本章不做

- 不把 `runtimeInferenceReady` 改成 `true`。
- 不做完整多语言自动路由；当前主流程先用 `rec-cjk`，因为它覆盖中文、英文、日文等截图常见场景。
- 不删除兼容 OCR 代码，但主流程不再执行外部 `PaddleOCR-json`。
- 不承诺识别质量已经完成最终调参；需要用户用真实截图继续验收。

### 验证

- `cargo fmt`：通过。
- `cargo check`：通过，无 warning。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。
- Python ONNX sanity：`rec-cjk.onnx` 可输出 `(1, 40, 18385)`，模型和字典均在项目根目录加载。

### 下一章建议

Chapter 103：真实截图调参与质量修正。

1. 用户用 `Ctrl+D` 截取真实文字区域，记录输出文本、截图预览和失败原因。
2. 根据真实结果调 detector threshold、crop padding、整图兜底、CJK/Latin recognizer 选择策略。
3. 如果英文技术文本仍不准，引入 `rec-latin` 作为识别 fallback，并按字符可信度选择最佳结果。
## Chapter 103：移除旧兼容配置入口并修 manifest / 白屏

### 目标

把当前用户可见路径收敛到“项目根目录 PP-OCRv5 ONNX 模型 + Ctrl+D 截图 OCR / 翻译”，停止让普通用户看到或点击 PaddleOCR-json / 兼容运行时配置。

### 修改文件

- `tauri-client/src-tauri/src/ysn_ocr_runtime.rs`
  - 修复 readiness 中误写入的换行字符，恢复 Rust 编译。
  - 模型根目录解析继续优先使用 `YSN_OCR_MODEL_ROOT` 和项目根目录 `models/ocr`。
- `tauri-client/src-tauri/src/ysn_ocr_manifest_store.rs`
  - manifest 解析错误带上完整路径。
  - 空 manifest 自动写回默认 manifest，避免 line 1 column 1 的无路径错误。
- `tauri-client/src/hooks/useOcrConfigController.tsx`
  - 删除配置页旧兼容 OCR 检查、下载、移动、选择目录等状态和调用。
  - 该 hook 只负责读取/保存产品配置，例如目标语言。
- `tauri-client/src/pages/OcrConfig.tsx`
  - 保留 PP-OCRv5 模型包、目标语言和视频录制依赖。
  - 不再传入兼容 OCR 状态或旧运行时路径。
- `tauri-client/src/components/config/OcrModelPackPanel.tsx`
  - 主按钮收敛为刷新、安装基础包、自测，不再打开“导入模型源 / 检测模型源”的文件浏览窗口。
  - 高级区也移除旧模型源浏览按钮，避免用户误以为需要手动找 PaddleOCR-json。
- `tauri-client/src/components/config/CompatibilityRuntimePanel.tsx`
  - 删除。
- `tauri-client/src/components/config/OcrRuntimePanel.tsx`
  - 删除。
- `tauri-client/src/components/config/types.ts`
  - 精简为当前仍需要的翻译和 FFmpeg 类型。
- `tauri-client/src/utils/ocrResultWindow.ts`
  - OCR 结果 payload 写入 `localStorage` 作为事件握手兜底。
- `tauri-client/src/pages/OcrPage.tsx`
  - 结果窗口先显示“正在加载 OCR 结果...”。
  - payload 事件丢失时从 `localStorage` 恢复；超时显示可操作提示，不再白屏。

### 本章不做

- 不把 `runtimeInferenceReady` 改成 `true`。
- 不承诺 OCR 质量已经最终调好；下一步仍要用真实截图调 detector threshold、crop padding 和 CJK/Latin fallback。
- 不继续扩展未来式配置面板。
- 后端遗留兼容函数暂不作为产品入口暴露；后续可在稳定后做代码级彻底删除。

### 用户测试路径

1. 重新构建并打开 `.exe`。
2. 进入“识字模型 / 视频录制”，点击“刷新”，确认模型目录指向项目根目录 `models/ocr`。
3. 按 `Ctrl+D` 框选清晰文字区域。
4. 结果窗口应至少显示截图预览、识别文本或明确错误；不应该再是白屏。
5. 如果仍报 manifest 错误，按错误里的完整路径检查对应 `manifest.json`，优先修项目根目录文件。

### 验证

- `cargo fmt`：通过。
- `cargo check`：通过。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。
- 根目录 `models/ocr/manifest.json`：开头为有效 JSON object。

### 下一章建议

Chapter 104：真实截图 OCR 质量调参。

1. 用用户当前失败截图复现 `Ctrl+D` 白屏/白图问题，确认是否仍是 payload、截图 base64、还是 detector 输出问题。
2. 调整 detector threshold、box filtering、crop padding 和整图兜底策略。
3. 对英文技术文本接入 `rec-latin` fallback，与 `rec-cjk` 结果按置信度选择。
4. 把翻译链路的低置信度 OCR 提示做成产品级状态，而不是静默失败。
## Chapter 104：Manifest BOM 修复

### 目标

解决用户当前阻塞的 `failed to parse OCR manifest ... expected value at line 1 column 1`，确保本地 OCR、翻译和模型包安装都不再因为 manifest 文件开头 BOM 失败。

### 根因

- `models/ocr/manifest.json` 文件开头存在 UTF-8 BOM：`EF BB BF`。
- Rust `serde_json::from_str` 不接受 BOM，因此在第 1 行第 1 列直接报 `expected value`。
- `scripts/install_ppocrv5_onnx_models.ps1` 使用 `Set-Content -Encoding UTF8`，在 Windows PowerShell 下会写出带 BOM 的 JSON，导致修完后可能再次复发。

### 修改文件

- `tauri-client/src-tauri/src/ysn_ocr_manifest_store.rs`
  - 读取 manifest 时先去掉 `U+FEFF`。
  - 如果 manifest 仍无法解析，自动备份为 `manifest.json.broken-*`，并写回默认 manifest，避免 OCR 主流程被永久卡死。
- `scripts/install_ppocrv5_onnx_models.ps1`
  - 新增 `Write-JsonUtf8NoBom`。
  - `manifest.json` 和 `active/installed-artifacts.json` 都改为 .NET UTF-8 no BOM 写入。
- `models/ocr/manifest.json`
  - 已去掉 BOM 并重新同步 PP-OCRv5 ONNX 模型安装状态。
- `models/ocr/active/installed-artifacts.json`
  - 已按 UTF-8 no BOM 重写。

### 验证

- `models/ocr/manifest.json` 首字节：`7B 0D 0A 20 20 20 20 22`，无 BOM。
- `models/ocr/active/installed-artifacts.json` 首字节：`5B 0D 0A 20 20 20 20 7B`，无 BOM。
- PowerShell `ConvertFrom-Json`：两个 JSON 均通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install_ppocrv5_onnx_models.ps1`：通过，未重复下载已有模型，已更新 manifest。
- `cargo fmt`：通过。
- `cargo check`：通过。
- `cargo test ysn_ocr_manifest_store`：通过，`6 passed`。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。

### 下一章建议

Chapter 105：在新构建 exe 中做真实 `Ctrl+D` OCR / 翻译人工验收，若仍失败，优先记录结果窗中的真实错误，不再从 manifest 方向排查。
## Chapter 105：Manifest 与模型状态深度检查

### 目标

在 Chapter 104 修复 BOM 后继续深查，确认 manifest 不会复发、模型文件与 SHA/size 一致、非必需扩展包失败不会阻塞基础 OCR。

### 检查结论

- 当前 `models/ocr/manifest.json` 无 BOM，首字节仍为 `7B 0D 0A 20 20 20 20 22`。
- 当前 `models/ocr/active/installed-artifacts.json` 无 BOM，首字节仍为 `5B 0D 0A 20 20 20 20 7B`。
- `auto-multilingual-balanced` 是唯一 required 基础包，状态为 `installed`。
- `accurate-extension` 是非 required 扩展包，当前 `download-failed` 不应阻塞截图 OCR 主流程。
- 14 个 active artifacts 的文件存在、size 与 manifest 一致、SHA256 与 manifest 一致。

### 修改文件

- `tauri-client/src-tauri/src/ysn_ocr_manifest_store.rs`
  - 抽出 `parse_manifest_content`，并新增 BOM JSON 解析单元测试。
- `tauri-client/src-tauri/src/ysn_ocr_runtime.rs`
  - 新增 `collect_required_broken_pack_ids`。
  - readiness 只让 required pack 的失败状态阻塞主流程，非必需扩展包失败不再把基础 OCR 判为不可用。
  - 新增单元测试覆盖 optional failed pack 不阻塞 required pack health。

### 验证

- Manifest/active artifacts Python 校验：通过，14 个 artifact 均存在且 SHA/size 一致。
- `cargo test parse_manifest_content_accepts_utf8_bom`：通过。
- `cargo test test_optional_failed_pack_does_not_block_required_pack_health`：通过。
- `cargo test`：通过，`103 passed`。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。

### 下一章建议

Chapter 106：启动新构建 exe 做真实 `Ctrl+D` OCR / 翻译人工验收；如果失败，优先依据结果窗真实错误继续修 detector/crop/recognizer，而不是 manifest。
## Chapter 106：OCR 结果窗白屏与速度修复

### 目标

解决用户反馈的 `Ctrl+D` 识别/翻译结果窗白屏，以及本地 PP-OCRv5 ONNX OCR 比旧流程慢很多的问题。

### 根因

- OCR 结果窗口按 `ocr_*` label 单独渲染 `OcrPage`，但没有包 `I18nProvider`；`OcrPage` / `OcrResultWindow` 调用 `useI18n()` 时会抛错，导致窗口只剩白底。
- `run_onnx_nchw_f32_outputs` 每次推理都会重新创建 ONNX session；一次 OCR 里 detector 加载一次，recognizer 对每个 crop 又重复加载，导致速度比旧兼容流程慢很多。
- detector 误检较多时最多处理 24 个 crop，进一步放大 recognizer 推理耗时。

### 修改文件

- `tauri-client/src/main.tsx`
  - 对 `ocr_*` 结果窗口包一层 `I18nProvider`，修复白屏。
- `tauri-client/src-tauri/src/ysn_ocr_runtime_adapter.rs`
  - 新增全局 ONNX session cache。
  - detector / recognizer session 按模型路径复用，避免每次截图、每个 crop 重复加载大模型。
- `tauri-client/src-tauri/src/lib.rs`
  - 单次 OCR 最大 detection/crop 从 24 降到 12，减少误检拖慢。
  - 修复部分 OCR 错误提示乱码。
- `tauri-client/src/i18n/dictionaries.ts`
  - 配置页文案从“ONNX 推理未启用”调整为“基础 ONNX 推理已接入，完整自测/fallback 未完成”，避免误导。

### 验证

- `cargo check`：通过。
- `cargo test`：通过，`103 passed`。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。

### 仍需真实验收

- 第一次 OCR 仍需要冷启动加载 detector/recognizer 两个 ONNX session；第二次开始应明显快。
- 如果仍慢，需要继续测真实截图的 detection 数量和每阶段耗时，再决定是否进一步降采样、预热模型或切换轻量 recognizer。
## Chapter 107：收敛为单一截图翻译能力

### 目标

响应用户明确目标：不要再让用户理解 YSN OCR Runtime / PP-OCRv5 / ONNX 多套概念，产品上只表现为一个“本地截图翻译模型”，优先保证截图顺畅、翻译能用、速度和准确率继续迭代。

### 修改文件

- `tauri-client/src-tauri/src/ysn_ocr_dictionary.rs`
  - 修复 PP-OCR 字典文件不含 CTC blank 时的 token 对齐问题。
  - 自动补 CTC blank token 和空格 token，避免 recognizer 类别与字典错位造成空识别或乱码。
- `tauri-client/src-tauri/src/lib.rs`
  - 用户可见错误改成“本地截图翻译模型未识别到文字”，不再混用 Runtime / PP-OCRv5 两套概念。
  - 新增 `prewarm_local_ocr_models`，启动后后台预热 detector / CJK recognizer / Latin recognizer。
- `tauri-client/src-tauri/src/ysn_ocr_runtime_adapter.rs`
  - 增加 ONNX session cache，避免每次截图、每个 crop 重复加载大模型。
- `tauri-client/src/App.tsx`
  - 主窗口启动后后台预热本地截图翻译模型。
- `tauri-client/src/i18n/dictionaries.ts`
  - 配置页文案收敛到“本地截图翻译模型”。

### 验证

- `cargo test ysn_ocr_dictionary`：通过，`6 passed`。
- `cargo test ysn_ocr_decode`：通过，`6 passed`。
- `cargo check`：通过。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。
- `cargo test`：通过，`104 passed`。

### 仍需真实验收

- 这些修复解决了白屏、模型重复加载、字典错位和概念混乱，但最终是否达到微信/QQ/pinpix 级速度与准确率，还必须用真实截图继续测每阶段耗时和识别结果。
- 如果仍慢，下一步优先做缩放策略、检测阈值和轻量 recognizer，不再扩展配置页。

## Chapter 108：截图翻译速度与白屏闭环修复

### 目标

继续响应用户反馈：当前翻译和识别比以前慢很多，`Ctrl+D` 后仍可能看到白屏/无有效结果。目标是先让截图翻译路径可用、可验证、速度不再被重复模型加载和错误回退拖垮。

### 修改文件

- `tauri-client/src-tauri/src/lib.rs`
  - 修复识别解码字典再次插入 CTC blank 的错位问题，改为使用已加载字典的 `blank_token_id`。
  - 在 CJK 识别为空或低置信度时才触发 Latin 识别回退，避免每行无条件双模型识别。
  - 将识别预处理配置移出 crop 循环，减少重复构造。
  - 增加 OCR 阶段耗时日志：decode、manifest、detector preprocess、detector inference、detector decode、crop、dictionary、recognition、detections、crops、fallbacks、blocks。
  - 修复后端用户可见错误提示乱码，继续统一为“本地截图翻译模型”。
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - 清理 `Ctrl+D` 识别和翻译失败弹窗里的旧架构术语。
  - 错误窗口直接提示重新框选真实文字区域，避免用户误以为还需要配置外部 OCR。

### 验证

- `cargo fmt`：通过。
- `cargo check`：通过。
- `cargo test`：通过，`104 passed`。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`491 zh-CN keys match 491 en-US keys`。
- `npm run build`：通过。

### 当前结论

- 白屏的已知前端崩溃点已在 Chapter 106 修复，本章继续清理错误窗口和文案，避免结果窗显示旧架构说明或乱码。
- 速度慢的主要代码级原因已修：ONNX session 缓存、启动预热、crop 上限、识别配置复用、Latin 只在低质量时回退。
- 如果用户新构建后仍慢，下一步直接看后台 `[local-screenshot-translate] ocr timings` 日志，优先处理 detector 输入尺寸和 crop 数量，不再继续堆配置页。

### 下一章建议

Chapter 109：用新构建 exe 做真实 `Ctrl+D` 截图翻译验收；记录实际耗时日志和预览图是否仍白。如果预览仍白，优先修截图源捕获；如果预览正常但识别慢，优先做 detector 降采样和阈值收敛。

## Chapter 109：移除旧 OCR 残留与当前可交接状态

### 目标

响应用户要求：不要再保留旧外部 OCR 路径，检查当前截图翻译是否正常可用、速度是否变快、翻译是否正确，并为换电脑继续操作留下可交接状态。

### 本章实际处理

- 移除产品运行时可触达的旧外部 OCR 命令入口：下载、选择目录、搬运运行包、检查旧运行包状态等不再注册到 Tauri invoke handler。
- 删除后端旧外部 OCR 进程管理、stdin/CLI JSON 解析和旧 fallback 代码，当前 `run_local_ocr` 只走项目根目录 `models/ocr` 下的本地截图翻译模型链路。
- 前端 `translateWithLocalOcr` 和 `Ctrl+D` 识别均不再传外部 OCR 路径，统一使用本地截图翻译模型。
- 删除/隐藏会打开文件浏览器的模型源导入和 inference probe 普通入口，配置页收敛为一个“本地截图翻译”产品卡片。
- 清理前端旧品牌残留和乱码文案，`rg` 已扫不到 `PaddleOCR`、`PaddleOCR-json`、`RapidOCR`、`localOcrExecutablePath`、`当前流程`、`OCR 暂不可用`、`OCR 状态` 等旧用户可见路径。
- 为模型大文件配置 Git LFS：`models/ocr/**/*.onnx`、`models/ocr/**/*.tar`、`models/ocr/**/*.pdiparams`。

### 速度检查结论

- 使用本机 Python `onnxruntime` 对当前模型做了直接基准：
  - `det-default.onnx`：加载约 `708ms`，热推理平均约 `2305ms`。
  - `rec-cjk.onnx`：加载约 `1680ms`，单行热推理平均约 `1268ms`。
  - `rec-latin.onnx`：加载约 `576ms`，单行热推理平均约 `19ms`。
- 基于该证据，本章把识别顺序调整为 Latin 快速模型优先，只有低质量时才回退 CJK 重模型；这会显著改善英文/拉丁字符截图速度。
- 对中文/日文等 CJK 截图，当前 `rec-cjk` 仍是明显瓶颈，还没有达到微信/QQ/pinpix 级极速；下一步必须做 detector 降采样、CJK 轻量模型或批处理/路由优化。

### 翻译正确性检查结论

- 当前已验证的是流程级正确性：OCR blocks → 语言选择 → 翻译请求 payload → 结果归一化 → 截图重绘。
- `npm run check:ocr-processing` 已通过，覆盖文本间距修复、技术词保护、源语言自动选择、翻译请求结构、短 UI 文本策略等。
- 尚未验证真实翻译服务返回质量；换电脑后需要用新构建 exe 截真实文本验证翻译语义是否正确。

### 验证

- `cargo check`：通过。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`489 zh-CN keys match 489 en-US keys`。
- `npm run build`：通过。
- 模型 manifest 检查：`models/ocr/manifest.json`、`models/ocr/active/installed-artifacts.json` JSON 可解析。

### 当前风险

- 模型大文件需要 Git LFS 正常推送和新电脑正常拉取，否则新电脑只有 LFS pointer 没有实际模型。
- 当前 CJK 大模型速度仍慢，不能宣称已达到“极速”。
- 当前没有做真实 exe 截图翻译人工验收；新电脑接手后第一件事是拉取 LFS 文件、构建新 exe、测试 `Ctrl+D` 和翻译按钮，并观察 `[local-screenshot-translate] ocr timings`。

### 下一章建议

Chapter 110：新电脑拉取仓库和 LFS 模型后，构建新 exe，做 3 类真实截图验收：英文 UI、中文大段、小字/多语言混合。若日志显示 detector 或 CJK recognizer 仍慢，优先改 detector 输入尺寸和 CJK 轻量模型路由。

## Chapter 110：新电脑 OCR 真实链路验收与热路径提速

### 目标

按 Chapter 109 交接要求，在新电脑拉取 LFS 模型后构建 release exe，验证本地截图翻译模型是否能启动、捕获、识别真实桌面文字，并优先处理实际暴露的模型健康和速度问题。

### 本章实际处理

- 修复 Windows checkout 后 OCR 字典 `.txt` 被 CRLF 转换导致 active artifact SHA/size 不一致的问题：
  - `.gitattributes` 新增 `models/ocr/active/dictionaries/*.txt -text`。
  - 6 个 active 字典恢复为 manifest 锁定的 LF 字节，SHA/size 重新匹配。
- 修复每次 OCR 前重复 SHA 校验整套 active 模型导致的热路径耗时：
  - 新增运行时轻量 artifact 检查，只校验安全路径、存在性、size 和 production source。
  - 完整 SHA 校验继续保留在启动诊断、配置健康检查和安装/修复路径。
- 抽出 `run_ysn_ocr_onnx_with_model_root_sync`，新增 ignored smoke 测试，可用 `YSN_OCR_MODEL_ROOT` + `YSN_OCR_SMOKE_IMAGE` 跑真实 crop 的 Rust OCR pipeline。
- 调整 line crop 策略：
  - OCR crop padding 从 4 提到 8。
  - 左侧 leading padding 额外放宽，缓解 detector 框偏右导致英文行首字符被吃掉。
- 截图缓存从 JPEG 80 改为 PNG：
  - `start_screenshot_impl` 写入 lossless PNG bytes。
  - `get_fullscreen_image` 和 `capture_region` 优先读取 PNG。
  - 前端截图页按 `data:image/png` 加载截图底图。
  - 写入新 PNG 时清理旧 `fullscreen_temp.jpg`。

### 新增文件

- 无。

### 修改文件

- `.gitattributes`
- `models/ocr/active/dictionaries/arabic.txt`
- `models/ocr/active/dictionaries/cjk.txt`
- `models/ocr/active/dictionaries/cyrillic.txt`
- `models/ocr/active/dictionaries/korean.txt`
- `models/ocr/active/dictionaries/latin.txt`
- `models/ocr/active/dictionaries/thai.txt`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/ysn_ocr_crop.rs`
- `tauri-client/src-tauri/src/ysn_ocr_model_downloader.rs`
- `tauri-client/src/pages/ScreenshotPage.tsx`

### 删除文件

- 无。

### 本章不做

- 不把 `runtimeInferenceReady` 改成 `true`。
- 不承诺 OCR 已达到微信/QQ/pinpix 级极速或准确率。
- 不恢复旧外部 OCR exe 路径。
- 不做 CDN、签名 source index、自动更新或模型回滚闭环。
- 不在普通配置页新增开发者调试面板。

### 真实验收结果

- 新电脑 LFS 模型已确认落盘，关键 ONNX 不是 pointer 文件。
- `check_commercial.ps1 -TauriBuild -SmokeLaunch`：通过。
- release exe 可启动并写入 startup diagnostics probe。
- 旧安装版会抢占 `Alt+A`，真实验收时必须先退出旧安装版，只保留当前构建 release exe。
- 当前构建 release exe 可注册并响应 `Alt+A`，生成 `YSN 截图辅助窗口`。
- Computer Use 无法对透明无边框 overlay 做坐标拖拽，原因是 Windows capture 对该窗口返回 `SetIsBorderRequired failed`；因此本章用应用真实全屏捕获文件裁出样例 crop，并通过 Rust smoke 入口验证 OCR pipeline。
- 新 release 已生成 `C:\Users\ysn\AppData\Local\ScreenshotTranslator\fullscreen_temp.png`，分辨率 `2560x1440`。

### OCR smoke 样例结果

样例 crop 包含：

- `Local OCR test`
- `PATH=C:\Windows\System32`
- `ONNX Runtime keeps .exe commands safe`
- `本地截图翻译测试`
- `混合 small text: OCR / Windows / PP-OCRv5`

优化前：

- Rust OCR smoke：约 `6621ms`。
- timings：`manifest=4708ms`，主要耗时来自每次 OCR 前 SHA 校验整套模型。
- 输出能识别 5 blocks，但英文行首和小字存在漏识别。

优化后：

- Rust OCR smoke：约 `1946ms` 到 `2050ms`。
- timings：`manifest=0ms`，`det≈1.1s`，`rec≈0.4s`。
- `PATH=C:\Windows\System32` 和 `本地截图翻译测试` 能稳定识别。
- 英文长句和小字仍有误读，例如 `Runtime/keeps/commands safe` 和 `PP-OCRv5` 仍不稳定。

### 验证

- `cargo fmt`：通过。
- `cargo check`：通过。
- `cargo test runtime_missing_check_skips_sha_hashing_but_keeps_strict_health`：通过。
- `cargo test ysn_ocr_crop::tests`：通过，`5 passed`。
- `cargo test smoke_local_ocr_image_from_env -- --ignored --nocapture`：通过，真实 crop 返回 5 blocks。
- `npm run check:i18n`：通过，`489 zh-CN keys match 489 en-US keys`。
- `npm run check:ocr-processing`：通过。
- `npm run build`：通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1 -TauriBuild -SmokeLaunch`：通过，`105 passed; 1 ignored; 0 failed`，release build 和 smoke launch 成功。

### 当前风险

- 真实 UI 框选后的 `Ctrl+D` 结果窗仍未由自动化完成验证，因为透明 overlay 无法被 Computer Use 坐标拖拽；用户或后续人工验收仍需手动框选一次确认结果窗。
- OCR 速度已明显改善，但 detector 仍约 `1.1s`，后续若追求极速，需要 detector 降采样/轻量 detector/预热策略。
- 小字和英文技术长句准确率仍不足，后续需要更强 Latin recognizer、基于字符集异常的 fallback scoring，或文本行二次放大/批处理。
- PNG 捕获改善了输入质量和避免 JPEG 损伤，但对当前小字样例准确率提升有限。

### 下一章建议

Chapter 111：继续真实样例 OCR 质量调参。优先做两件事：

1. 为 Latin 识别加入字符集/技术词异常评分，遇到 `Runtie/keps/comands/P-OCV` 这类高置信错误时触发更合适的 fallback 或二次识别。
2. 建立 3 个固定 smoke crop fixture：英文 UI、中文大段、小字混合技术文本，把输出文本和耗时写成可重复测试，避免每次只靠目测。

## Chapter 111：翻译失败门禁与覆盖重绘对齐修复

### 目标

响应新电脑真实使用反馈：当前截图翻译存在“没有翻译成功但界面像成功了”的问题，并且翻译回填覆盖层在小选区、文件名/技术标识等场景里看起来没有对齐。先修复用户可感知的翻译/重绘主流程，再继续后续 OCR fixture 工作。

### 本章实际处理

- 前端翻译链路新增质量门禁：
  - 识别可翻译文本、已翻译文本、技术标识保留文本、空译文和未翻译文本。
  - 普通英文 UI 文案如果翻译服务返回原文或空字符串，不再静默当作成功，而是抛出明确错误。
  - 文件名、路径、命令参数、包名、全大写下划线标识、`.exe/.onnx/.md/.json` 等技术文本可按规则保留，并在 UI 里提示“已识别但按技术标识保留”。
- 翻译结果对话文本新增状态标记，能区分“已翻译 / 已保留技术标识 / 未返回有效译文”。
- 服务端翻译失败兜底改为返回空译文而不是原文，避免把真实通道失败伪装成成功译文或写入缓存。
- 覆盖重绘修复：
  - 译文等于原文的块不再擦除重画，避免文件名/路径被重新绘制后错位。
  - LTR 文本扩展区域改为从原 OCR 左边界向右扩展，RTL 从右边界向左扩展，避免对称扩展导致横向漂移。
  - 小高度选区使用更稳的字体缩放、单行约束和省略策略，避免文字溢出蓝框。
  - 字号下限从 10/11px 放宽到 7/8px，适配 20px 左右的小字选区。
- 检查脚本补充翻译质量 fixture，锁住“普通英文必须翻译、技术文件名应保留、未翻译英文必须失败”的行为。
- 服务端测试补充翻译通道完全失败时返回空译文的用例。

### 新增文件

- 无。

### 修改文件

- `server/app.py`
- `server/translator.py`
- `server/tests/test_translate_text.py`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/translation-render/renderTranslatedBlocks.ts`
- `tauri-client/src/translation-render/textLayout.ts`
- `tauri-client/src/types/screenshot.ts`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改变 OCR Runtime ready 状态。
- 不恢复旧外部 OCR exe 路径。
- 不把纯文件名、路径、命令、包名强行翻译成中文，因为这会违反技术标识保护要求。
- 不建立新的长期计划文档。
- 不完成 Chapter 110 原计划中的固定 OCR crop fixture；该工作顺延到下一章。

### 验证

- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`489 zh-CN keys match 489 en-US keys`。
- `npm run build`：通过。
- `python -m pytest server\tests\test_translate_text.py`：通过，`4 passed`。
- `.\check_commercial.ps1 -TauriBuild`：
  - 第一次失败原因是旧 `target\release\tauri-client.exe` 正在运行，Windows 拒绝删除构建产物。
  - 结束占用进程后重跑通过，release exe、MSI 和 NSIS installer 均生成成功。
- `.\check_commercial.ps1 -SmokeLaunch`：通过，release exe 可启动并写入 startup diagnostics probe。

### 当前风险

- 本章修复了“翻译失败被伪装成成功”和“未变化文本重绘错位”的代码级问题，但仍需要用户手动用透明 overlay 框选真实文本验收，因为 Computer Use 仍无法可靠拖拽该无边框截图窗口。
- 纯技术标识被保留时，画面看起来不会变化；现在会有提示和结果状态，但用户如果希望翻译文件名语义，需要后续单独设计“说明性翻译/不覆盖原文”的高级策略。
- OCR 小字识别和英文长技术句准确率仍是 Chapter 110 遗留风险，本章没有解决 recognizer 误读。

### 下一章建议

Chapter 112：回到 Chapter 110 原计划，建立固定 OCR smoke crop fixture，并增加 Latin 技术词异常评分。优先覆盖英文 UI、中文大段、小字/技术词混合三类样例，把识别文本和阶段耗时变成可重复门禁。

## Chapter 112：翻译自动化 smoke 修复

### 目标

解释并修复当前桌面自动化无法稳定拖拽透明截图层的问题，让“大小不同、行数不同、技术标识保留”的翻译验证可以在用户电脑上重复运行，而不依赖 Chrome 页面或透明 overlay 坐标拖拽。

### 本章实际处理

- 明确自动化失败原因：
  - Chrome 本地测试页会触发 Computer Use 的浏览器 URL 安全检查；URL 无法确认时工具会停止。
  - 透明无边框截图 overlay 触发 Windows capture `SetIsBorderRequired failed: 不支持此接口`，无法可靠截图/拖拽。
- 新增可重复的真实翻译服务 smoke：
  - `npm run smoke:translate-service`
  - 读取 `%LOCALAPPDATA%\ScreenshotTranslator\config.json` 中的 `serverUrl`、`clientToken`、`targetLang`。
  - 覆盖小短句、中等句、多行文本、技术文件名、技术命令/路径。
  - 普通英文必须返回中文且不能等于原文；技术标识允许保留原文。
- 覆盖渲染逻辑抽出纯几何模块：
  - `shouldRenderTranslationBlock`：译文等于原文时不擦除重画。
  - `buildTranslationEraseRegion`：验证 LTR 从左边界向右扩展、RTL 从右边界向左扩展。
- 将覆盖几何断言并入 `npm run check:ocr-processing`，在无需透明窗口的情况下锁住 Chapter 111 的对齐修复。

### 新增文件

- `tauri-client/scripts/smoke-translate-service.mjs`
- `tauri-client/src/translation-render/renderGeometry.ts`

### 修改文件

- `tauri-client/package.json`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/src/translation-render/renderTranslatedBlocks.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不引入 Playwright、Puppeteer、sharp 或其它新依赖。
- 不用 PowerShell SendKeys 绕过 Computer Use 的停止规则。
- 不宣称透明 overlay 自动化已完全可拖拽；本章是建立可靠替代门禁。
- 不解决 OCR 小字识别准确率；该项仍按下一章继续。

### 验证

- `npm run check:ocr-processing`：通过。
- `npm run smoke:translate-service`：通过。
  - `Open preview => 打开预览`
  - `Translate selected text and keep commands safe => 翻译所选文本并保证命令安全`
  - 多行文本返回三行中文译文。
  - `COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 保留原文。
  - `PATH=C:\Windows\System32 && LocalModel.exe --help` 保留原文。
- `npm run build`：通过。
- `python -m pytest server\tests\test_translate_text.py`：通过，`4 passed`。

### 当前风险

- 真实透明 overlay 框选仍需要人工验收或后续专门做截图窗口可测试性改造。
- 新 smoke 验证的是翻译服务与覆盖几何纯逻辑，不直接验证 OCR 识别准确率。
- 技术标识保留策略会让纯文件名/命令看起来“没有变化”，但现在自动化和 UI 状态都会把它作为保留而非失败。

### 下一章建议

Chapter 113：继续 Chapter 110 原计划的 OCR 固定 crop fixture 和 Latin 技术词异常评分，重点解决小字和英文长技术句误识别，而不是继续扩展翻译服务 smoke。

## Chapter 113：多语言翻译 smoke 与源语言路由修复

### 目标

按用户追问补测更细的翻译场景：小型短句、单个词、半句、中英混合、韩文、阿拉伯语、日文、法语、西语、多行文本，以及记录翻译速度。修复测试中暴露的源语言路由问题。

### 本章实际处理

- 扩展 `npm run smoke:translate-service`：
  - 小短句：`Open preview`
  - 单个词：`Save`
  - 半句：`Open preview and`
  - 中英混合：`打开 preview before saving`
  - 韩文：`파일을 저장하세요`
  - 阿拉伯语：`افتح المعاينة قبل الحفظ`
  - 日文：`保存する前にプレビューを開く`
  - 法语：`Ouvrir l'aperçu avant d'enregistrer`
  - 西语：`Abrir vista previa antes de guardar`
  - 中等英文、多行英文、技术文件名、技术命令/路径。
- smoke 脚本按自动脚本 hint 分批请求翻译服务，输出每批耗时和总耗时。
- 修复前端源语言选择：
  - 只有纯 Latin/英文类批次才优先 `en`。
  - 如果混入韩文、阿拉伯、日文、俄文、泰文等非 Latin 脚本，保持 `auto`，避免整批误判为英文。
- 修复服务端 Google 通道：
  - 对韩文、阿拉伯语、日文、俄文、泰文自动注入 `ko/ar/ja/ru/th` source hint。
  - `source_lang=auto` 且包含混合脚本 hint 时，跳过 Google 批量合并请求，降级为逐条翻译，避免某一种脚本拖坏整批。
- 服务端测试新增韩文 source hint 覆盖，防止韩文继续用 `auto` 缓存原文。

### 新增文件

- 无。

### 修改文件

- `server/translator.py`
- `server/tests/test_translate_text.py`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改成用户手动选择源语言；源语言仍由内部脚本检测自动决定。
- 不承诺 Google 免费通道的所有语言语义质量已达商业级。
- 不解决 OCR 识别多语言准确率，只验证文本翻译服务链路。
- 不做翻译服务线上部署流程；本地代码已修，远端服务需要用同版本重启/部署才能获得服务端 hint 修复。

### 测试中发现的问题

- 初始扩展 smoke 中：
  - 韩文 `파일을 저장하세요` 在 Google 免费通道 `source_lang=auto` 下原样返回。
  - 阿拉伯语在错误源语言下曾返回异常符号。
  - 中英混合如果走 `auto` 也会原样返回。
- 修复后：
  - 韩文通过 `ko` hint 返回 `保存您的文件`。
  - 阿拉伯语通过 `ar` hint 返回中文译文。
  - 日文通过 `ja` hint 返回中文译文。
  - 中英混合按客户端真实策略走 `en`，返回 `保存前打开预览`。

### 真实 smoke 结果

`npm run smoke:translate-service`：通过。

- `source=en`：10 blocks，约 `1092ms`。
- `source=ko`：1 block，约 `1717ms`。
- `source=ar`：1 block，约 `250ms`。
- `source=ja`：1 block，约 `241ms`。
- 总计：13 blocks、4 batches，约 `3300ms`。

通过项：

- `Open preview => 打开预览`
- `Save => 保存`
- `Open preview and => 打开预览并`
- `打开 preview before saving => 保存前打开预览`
- `파일을 저장하세요 => 保存您的文件`
- `افتح المعاينة قبل الحفظ => 保存前打开预览`
- `保存する前にプレビューを開く => 保存前打开预览`
- `Translate selected text and keep commands safe => 翻译所选文本并保证命令安全`
- 多行英文返回 3 行中文译文。
- 技术文件名和技术命令/路径正确保留。

质量警告：

- 法语和西语本次只是“返回了中文且非原文”，但语义质量不理想：
  - 法语返回 `请注意注册前的操作`，语义不准。
  - 西语返回 `瓜达尔的前夕`，语义明显错误。
- 因此 Google 免费通道不能作为商业级多语言质量结论；后续要优先接入更可靠 LLM/付费翻译通道，或者对非英文语言设置质量检测和重试。

### 验证

- `npm run check:ocr-processing`：通过。
- `npm run smoke:translate-service`：通过。
- `npm run build`：通过。
- `python -m pytest server\tests\test_translate_text.py`：通过，`5 passed`。

### 当前风险

- 远端 `https://ocr.yousn.me` 当前运行的服务端是否已包含本章 `server/translator.py` 修复，取决于部署/重启；本地脚本用分批 source hint 可通过，但真实 app 若调用远端旧服务，服务端 hint 需要部署后才完整生效。
- Google 免费通道对法语/西语语义质量不可靠，不能作为商业级多语言翻译方案。
- 本章没有验证 OCR 对韩文/阿拉伯语截图的识别能力，只验证翻译文本服务。

### 下一章建议

Chapter 114：建立翻译通道质量策略。优先做非英文语言的质量检测、Google 免费通道降级警告、LLM/付费通道优先级，以及真实 OCR crop fixture，不要把“返回中文”误当成“语义正确”。

## Chapter 114：家里内网翻译服务优先与公网回落

### 目标

按用户说明，`ocr.yousn.me` 是用户自己的 N100 家庭服务器，经 FRP 到阿里云香港；在家时应优先走内网地址，减少公网/FRP 链路，并在离家或内网不可用时自动回落 `ocr.yousn.me`。

### 本章实际处理

- 设置页新增家里内网翻译服务配置：
  - `lanServerUrl`：家里 N100 内网地址，例如 `http://192.168.x.x:8318`。
  - `preferLanServer`：优先使用家里内网服务，失败后回落公网。
- 截图翻译请求链路新增双地址候选：
  - 启用 `preferLanServer` 且填写 `lanServerUrl` 时，先请求内网地址。
  - 内网请求失败、超时或返回错误时，自动请求公网 `serverUrl`。
  - retry 未翻译 Latin blocks 时也复用同一候选地址列表。
- 设置保存同步服务端通道配置时也支持双地址候选：
  - 在家优先把百度/New API/Google active channel 保存到内网 N100。
  - 内网不可用时再同步公网服务端。
- 顶部服务状态检查改为显示当前优先服务地址：
  - 在家启用内网优先时，状态栏检查内网 N100。
  - 未启用或未填写内网地址时，仍检查公网地址。
- `npm run smoke:translate-service` 跟随同一策略：
  - 配置启用内网优先时测试内网服务。
  - 否则测试公网服务。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/components/settings/TranslationServiceCard.tsx`
- `tauri-client/src/hooks/useServerStatus.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/pages/Settings.tsx`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/src/utils/ocrConfigHelpers.ts`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不移除 `ocr.yousn.me`，它仍作为公网/离家回落服务。
- 不实现客户端直连百度；当前仍由翻译服务端管理百度/New API/Google 通道。
- 不改变 OCR Runtime。
- 不解决 Google 免费通道法语/西语语义质量问题。

### 使用方式

在设置页“翻译服务”里：

- 文本翻译服务地址：继续填 `https://ocr.yousn.me`。
- 家里内网服务地址：填 N100 的局域网地址，例如 `http://192.168.1.10:8318`。
- 勾选“优先使用家里内网服务，失败后回落公网”。

保存后，新截图翻译会优先打内网 N100；离家或内网不通时自动回落公网地址。

### 验证

- `npm run check:i18n`：通过，`492 zh-CN keys match 492 en-US keys`。
- `npm run build`：通过。
- `npm run smoke:translate-service`：通过；当前未配置内网地址，仍走 `https://ocr.yousn.me`，13 blocks / 4 batches 约 `2390ms`。
- `npm run check:ocr-processing`：通过。
- `python -m pytest server\tests\test_translate_text.py`：通过，`5 passed`。

### 当前风险

- 尚未填入真实 N100 内网地址做内网测速；本章验证的是配置、构建、公网 smoke 和回落逻辑。
- 如果内网服务和公网服务配置不同，保存通道配置只会同步到第一个成功的服务；建议内网 N100 和公网 FRP 指向同一台服务，或后续加“一键同步所有服务”。
- 翻译服务密钥仍保存在服务端，不在客户端直连第三方翻译平台。

### 下一章建议

Chapter 115：在用户提供 N100 内网地址后跑内网/公网 A-B 测速，记录客户端到服务端延迟、服务端翻译耗时、缓存命中率；然后做持久化缓存和短 UI 词典，真正把常见截图翻译压到毫秒级/亚秒级。

## Chapter 115：N100 内网地址实测、启用与自动化 BOM 修复

### 目标

用户提供候选内网地址 `192.168.1.3` / `192.168.1.6`，端口不确定。本章目标是从本机实际探测 N100 翻译服务地址，启用内网优先配置，并验证内网相比公网 `ocr.yousn.me` 的真实速度收益。

### 本章实际处理

- 从 N100 笔记确认：
  - `192.168.1.3` 是 N100 主机地址。
  - `192.168.1.6` 更像 OpenWrt / 网关 VM 地址。
  - 项目服务默认端口是 `8318`，来自 `server/app.py` 的 `SS_TRANSLATOR_PORT` 默认值。
- 端口探测结果：
  - `192.168.1.3:8318` 开放，并且 `/api/health` 返回 `{"status":"ok","ocr":"client-local-only"}`，确认是本项目翻译服务。
  - `192.168.1.6:80` 开放，但 `/api/health` 返回 404，不是翻译服务本体。
- 已更新本机应用配置 `%LOCALAPPDATA%\ScreenshotTranslator\config.json`：
  - `lanServerUrl` 设置为 `http://192.168.1.3:8318`。
  - `preferLanServer` 设置为 `true`。
  - `serverUrl` 保留 `https://ocr.yousn.me`，作为离家或内网不可用时的公网回落。
- 修复 `npm run smoke:translate-service` 读取 Windows 配置文件时遇到 UTF-8 BOM 会 `JSON.parse` 失败的问题：
  - 读取配置时去掉开头 `\uFEFF`。
  - 避免 PowerShell 或其他 Windows 工具重写配置后打断自动化 smoke。

### 新增文件

- 无。

### 修改文件

- `tauri-client/scripts/smoke-translate-service.mjs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改第三方翻译通道；当前仍是服务端 Google 通道。
- 不部署/重启 N100 服务端代码。
- 不把客户端改成直连百度或其他第三方平台。
- 不解决 Google 免费通道法语/西语语义质量问题。

### 验证

- `http://192.168.1.3:8318/api/health`：通过，确认是翻译服务。
- `http://192.168.1.3:8318/api/config/current`：带 token 通过，当前 active channel 是 `google`。
- `https://ocr.yousn.me/api/config/current`：带 token 通过，当前 active channel 是 `google`。
- `npm run smoke:translate-service`：通过，确认走内网 `http://192.168.1.3:8318`。
  - 缓存命中场景：13 blocks / 4 batches 合计约 `40ms`。
  - 覆盖小型 UI、单词、半句、中英混合、韩文、阿拉伯语、日文、多行英文、技术文件名、技术命令/路径。
- 未缓存单句 A-B 测速：
  - 内网 `http://192.168.1.3:8318`：约 `465ms`。
  - 公网 `https://ocr.yousn.me`：约 `1029ms`。
  - 本次内网约快 `564ms`，省掉公网/FRP/香港链路和额外抖动。
- `npm run check:i18n`：通过，`492 zh-CN keys match 492 en-US keys`。
- `npm run check:ocr-processing`：通过。

### 当前风险

- 本机配置文件里旧字段 `paddleOcrReleaseCheckedAt` 本来已有乱码时间值；本章没有依赖该字段。
- 内网 smoke 很快主要因为已有服务端缓存；未缓存请求仍受 Google 免费通道耗时影响，不能只靠内网解决所有慢的问题。
- N100 服务端当前是否包含最新 `server/translator.py` 修复，取决于服务端实际部署版本；本章只确认当前运行服务可用且配置已切到内网优先。
- 法语/西语仍存在 Google 免费通道语义质量风险，不能作为商业级多语言结论。

### 下一章建议

Chapter 116：做翻译加速闭环。优先实现客户端持久化翻译缓存、短 UI 词典/术语表、批量请求耗时分解日志，并把 smoke 输出拆成 `client->server latency`、`provider duration`、`cache hits`，这样才能判断每次慢到底是 OCR、网络、服务端缓存、还是翻译供应商。

## Chapter 116：客户端翻译加速闭环与服务端耗时拆分

### 目标

在 Chapter 115 已确认家里内网服务可用后，继续把常见截图翻译从“依赖服务端缓存很快”推进到“客户端也能本地毫秒级命中”。同时让服务端返回耗时拆分，后续能判断慢点来自 OCR、客户端到服务端、服务端缓存、还是第三方翻译供应商。

### 本章实际处理

- 新增客户端翻译记忆模块：
  - 短 UI 词典：对 `Save`、`Open preview`、`Copy translated text`、`Check OCR result window` 等高频英文 UI 文案直接本地返回中文。
  - 技术/无需翻译文本本地保留：文件名、路径、命令、环境变量赋值、`PATH`、`Windows`、`.exe` 等不再进入翻译请求。
  - 持久化翻译缓存：成功译文写入浏览器 `localStorage`，按 `source text + sourceLang + targetLang + channel + version` 建 key。
  - 缓存容量限制为 `1000` 条，TTL 为 `30` 天；写入失败会清理缓存，避免坏缓存拖垮主流程。
- 截图翻译链路改为分层请求：
  - OCR blocks 先过本地保留/短词典/持久缓存。
  - 只把未命中的 blocks 发给翻译服务。
  - 服务端返回后合并回原 block 顺序，再走原来的翻译质量门禁和覆盖渲染。
  - 成功译文写回持久缓存，下一次相同文本可本地命中。
- 修复一个技术文本 retry 风险：
  - Latin retry 现在必须先通过 `shouldRequireTranslation`。
  - `PATH=C:\Windows\System32 && LocalModel.exe --help` 这类命令行不会再被 retry 发回翻译服务。
- 服务端 `/api/translate_text` 新增 `timings` 字段：
  - `total_ms`
  - `provider_ms`
  - `cache_hits`
  - `provider_misses`
  - `blocks`
  - 保留旧字段 `cache_hits` 和 `channel`，兼容旧客户端。
- `npm run smoke:translate-service` 输出增强：
  - 原 `duration` 改为 `client=...ms`。
  - 如果服务端已经部署新代码，会额外显示 `server/provider/cache/miss`。
  - 如果 N100 仍是旧服务端，只显示已有 `cache`，不影响 smoke 通过。

### 新增文件

- `tauri-client/src/utils/translationMemory.ts`

### 修改文件

- `server/app.py`
- `server/tests/test_translate_text.py`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改变 OCR Runtime ready 状态。
- 不把客户端改成直连百度、Google 或 LLM。
- 不替换当前 N100 服务端运行版本；服务端 timing 字段需要部署/重启 N100 后才会在线上响应里出现。
- 不解决 Google 免费通道法语/西语语义质量问题。

### 验证

- `npm run check:ocr-processing`：通过。
  - 覆盖短 UI 词典：`Save => 保存`。
  - 覆盖持久翻译缓存写入和二次命中。
  - 覆盖技术文本本地保留。
  - 覆盖 Latin retry 不再发送 protected command/path。
- `npm run build`：通过。
- `npm run check:i18n`：通过，`492 zh-CN keys match 492 en-US keys`。
- `python -m pytest server\tests\test_translate_text.py`：通过，`5 passed`。
- `python -m pytest server\tests`：通过，`20 passed, 1 skipped`。
- `npm run smoke:translate-service`：通过，当前走内网 `http://192.168.1.3:8318`。
  - `source=en`：10 blocks，client 约 `36ms`，cache `10`。
  - `source=ko`：1 block，client 约 `3ms`，cache `1`。
  - `source=ar`：1 block，client 约 `2ms`，cache `1`。
  - `source=ja`：1 block，client 约 `2ms`，cache `1`。
  - 总计：13 blocks / 4 batches，约 `43ms`。

### 当前风险

- N100 当前运行服务端还没有确认部署 Chapter 116 的 `timings` 字段；live smoke 只显示旧服务端已有的 `cache`。
- 客户端持久缓存会让重复文本极快，但如果第三方通道曾返回错误译文，缓存会保留 30 天；后续需要增加“清理翻译缓存”和通道版本升级失效策略。
- 短 UI 词典只覆盖明确高频英文 UI 文案，不覆盖复杂句、多语言长句或需要上下文的内容。
- Google 免费通道对法语/西语仍不可靠；本章提升速度和可观测性，不把它标记为商业级质量通道。

### 下一章建议

Chapter 117：做翻译缓存 UX 和部署闭环。优先加“清理翻译缓存 / 查看缓存命中状态”，并把 Chapter 116 服务端代码部署到 N100 后重新跑公网/内网 A-B，确认 `timings.total_ms/provider_ms/cache_hits/provider_misses` 在线上真实返回。

## Chapter 117：翻译缓存恢复路径 UX

### 目标

Chapter 116 引入客户端持久化翻译缓存后，必须给用户一个清楚的恢复路径：如果旧译文或错译文被缓存，用户应能在设置页看到缓存状态并一键清理，而不是只能手动找 localStorage。

### 本章实际处理

- `translationMemory` 新增缓存管理 API：
  - `getTranslationMemoryStorageStats()`：返回当前缓存条数、容量上限和 TTL 天数。
  - `clearTranslationMemory()`：清理本机翻译缓存。
- 设置页“翻译服务”卡片新增本机翻译缓存区域：
  - 显示当前条数，例如 `0/1000`。
  - 显示有效期 `30` 天。
  - 提供“刷新”和“清理缓存”按钮。
  - 清理完成后显示成功提示。
- i18n 增加中英文文案，保持 `check:i18n` 门禁一致。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/components/settings/TranslationServiceCard.tsx`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/utils/translationMemory.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不实现缓存逐条查看/编辑。
- 不把缓存迁移到 Tauri 文件数据库；当前仍使用浏览器 `localStorage`。
- 不部署 N100 服务端 timing 字段；该项仍留到下一章。

### 验证

- `npm run check:i18n`：通过，`498 zh-CN keys match 498 en-US keys`。
- `npm run build`：通过。
- `npm run check:ocr-processing`：通过。

### 当前风险

- 设置页能清理本机缓存，但不能定位是哪一条译文导致问题；如果后续缓存规模变大，需要做缓存详情或诊断导出。
- N100 当前运行服务端是否有 Chapter 116 timing 字段仍未确认部署。

### 下一章建议

Chapter 118：部署/同步 N100 服务端 timing 字段并重跑内网、公网 A-B；如果无法自动部署，则先补本地一键 server smoke，用本机服务确认 `timings` 字段，再整理 N100 部署清单。

## Chapter 118：N100 服务端 timing 部署与线上 A-B 验证

### 目标

把 Chapter 116 的服务端 `timings` 字段和多语言/短 UI 词典修复部署到 N100 当前运行服务，并用内网与公网真实请求确认线上返回 `total_ms/provider_ms/cache_hits/provider_misses`。

### 本章实际处理

- 通过 N100 笔记和只读 SSH 探测确认：
  - 服务目录：`/vol1/1000/项目/自制截图/server`。
  - 运行命令：`.venv/bin/python -m uvicorn app:app --host 0.0.0.0 --port 8318`。
  - 原进程：`567309`，后续重启为 `2073598`、`2074999`、最终 `2075851`。
- 远端部署前备份：
  - `deploy-backups/20260603-001921/`：备份 `app.py` 和 `translator.py`。
  - `deploy-backups/20260603-002246/`：备份 `translator.py`。
- 同步到 N100：
  - `server/app.py`
  - `server/translator.py`
- 远端验证：
  - `.venv/bin/python -m py_compile app.py translator.py` 通过。
  - `http://192.168.1.3:8318/api/health` 通过。
  - `/api/translate_text` 线上返回 `timings` 字段。
- 修复部署 smoke 中暴露的两个服务端翻译质量问题：
  - 多行 block 不能走 Google 换行拼批策略，否则会把译文压成一行；现在遇到 block 内 `\n` 时降级逐条并发翻译，保留行结构。
  - `Save` 不能依赖 Google 单词直译，否则可能返回“节省”；服务端新增短 UI 词典，返回“保存”。
- `npm run smoke:translate-service` 增强后的多行断言已覆盖“多行必须保留多行”。

### 新增文件

- 无。

### 修改文件

- `server/translator.py`
- `server/tests/test_translate_text.py`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 N100 的 OpenResty / FRP 配置。
- 不改第三方翻译通道；当前线上仍是 Google 通道。
- 不处理远端旧测试 `tests/test_processor.py` 依赖缺失问题；该测试缺 `image_processor`，不属于本章服务端 API 热路径。

### 验证

- 本地 `python -m pytest server\tests`：通过，`21 passed, 1 skipped`。
- 本地 `python -m pytest server\tests\test_translate_text.py`：通过，`6 passed`。
- 本地 `npm run check:ocr-processing`：通过。
- N100 `.venv/bin/python -m py_compile app.py translator.py`：通过。
- N100 `http://192.168.1.3:8318/api/health`：通过。
- N100 `npm run smoke:translate-service`：通过。
  - `source=en`：10 blocks，client 约 `1448ms`，server `1413ms`，provider `1413ms`，cache `3`，miss `7`。
  - `source=ko`：1 block，client 约 `171ms`，server `166ms`。
  - `source=ar`：1 block，client 约 `131ms`，server `128ms`。
  - `source=ja`：1 block，client 约 `164ms`，server `160ms`。
  - `Save => 保存`。
  - 多行英文保持 3 行中文译文。
- 未缓存新文本 A-B：
  - 内网 `http://192.168.1.3:8318`：client 约 `337ms`，server total `288ms`，provider `288ms`，cache `0`，miss `1`。
  - 公网 `https://ocr.yousn.me`：client 约 `1419ms`，server total `485ms`，provider `485ms`，cache `0`，miss `1`。

### 当前风险

- 远端重启命令在 SSH 中会因后台进程继承会话偶发超时，但服务实际已启动；后续应做一个稳定的 N100 restart 脚本或 systemd user service。
- Google 免费通道仍不是商业级多语言质量方案，法语/西语 smoke 仍只是“返回中文”而非语义正确。
- 服务端短 UI 词典只覆盖明确高频 UI 文案，不能替代完整翻译质量策略。

### 下一章建议

Chapter 119：整理 N100 服务部署脚本，避免手动 kill/nohup/setsid；然后回到 OCR 固定 crop fixture，把英文 UI、中文大段、小字技术文本变成可重复 OCR 质量门禁。

## Chapter 119：N100 翻译服务部署脚本固化

### 目标

Chapter 118 手动部署 N100 时暴露出 SSH 后台进程、中文路径编码、手动 kill/nohup/setsid 容易出错的问题。本章目标是把部署流程固化成一个可重复脚本，包含备份、上传、语法检查、重启、health 和 timing smoke。

### 本章实际处理

- 新增根目录部署脚本 `deploy_n100_translation_server.ps1`：
  - 远端备份 `app.py` 和 `translator.py` 到 `deploy-backups/<timestamp>/`。
  - 上传本地 `server/app.py` 和 `server/translator.py`。
  - 远端执行 `.venv/bin/python -m py_compile app.py translator.py`。
  - 重启 `8318` 端口的 uvicorn。
  - 检查 LAN health。
  - 对 LAN 和可选公网执行 `/api/translate_text` timing smoke。
- 解决 Windows PowerShell 5 中文路径乱码：
  - 在 N100 上创建 ASCII 软链接 `/home/ysn/screenshot-translator-server` 指向 `/vol1/1000/项目/自制截图/server`。
  - 部署脚本默认使用该 ASCII 路径。
- 解决 `pkill -f` 误杀当前 SSH 命令的问题：
  - 改为 `ps | grep '[u]vicorn app:app' | awk pid | xargs kill`。
- 解决远端后台启动导致 SSH 不退出的问题：
  - 使用 `Start-Process ssh -WindowStyle Hidden` 发起启动。
  - 等待后若本地 ssh 壳未退出，则强制结束本地 ssh 壳；远端 uvicorn 已由 `setsid -f` 脱离。
- 最终确认 N100 只剩一个 `8318` uvicorn 进程。

### 新增文件

- `deploy_n100_translation_server.ps1`

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 OpenResty / FRP。
- 不改成 systemd service；当前是脚本化部署，systemd 可作为后续增强。
- 不提交或推送代码。

### 验证

- `powershell -NoProfile -ExecutionPolicy Bypass -File .\deploy_n100_translation_server.ps1 -SkipPublicSmoke`：通过。
  - 远端备份：`deploy-backups/20260603-004116`。
  - LAN health：`200 {"status":"ok","ocr":"client-local-only"}`。
  - LAN timing smoke：client 约 `1331ms`，server total `1279ms`，provider `1279ms`，cache `0`，miss `1`。
- N100 进程检查：只剩一个 `8318` uvicorn，pid `2083662`。
- `npm run check:i18n`：通过，`498 zh-CN keys match 498 en-US keys`。
- `npm run build`：通过。
- `python -m pytest server\tests`：通过，`21 passed, 1 skipped`。
- `npm run smoke:translate-service`：通过。
  - `source=en`：10 blocks，client 约 `494ms`，server `457ms`，provider `457ms`，cache `3`，miss `7`。
  - `source=ko`：1 block，client 约 `164ms`。
  - `source=ar`：1 block，client 约 `134ms`。
  - `source=ja`：1 block，client 约 `237ms`。
  - `Save => 保存`。
  - 多行英文保持多行中文译文。

### 当前风险

- 部署脚本仍依赖 `ssh n100` 免密配置和 N100 上的 `.venv` 已存在。
- 脚本使用 `Start-Process ssh` 启动远端服务并在必要时结束本地 ssh 壳；当前实测可用，但长期更推荐 systemd user service 或 supervisord。
- Google 免费通道的法语/西语语义质量仍不可靠。

### 下一章建议

Chapter 120：回到 OCR 固定 crop fixture。优先固化英文 UI、中文大段、小字技术文本三类真实图片样例，记录 OCR 输出、耗时和失败原因，为后续 detector/recognizer 优化提供稳定门禁。

## Chapter 120：Latin 多语种源语言路由修复

### 目标

用户要求继续优化翻译系统。本章优先处理 Chapter 113 以来遗留的真实质量问题：法语/西语以前只是“返回中文”，但语义明显错误。根因是当前路由把所有 Latin 字母文本都偏向 `source_lang=en`，导致法语/西语被错误按英文送入 Google。

### 本章实际处理

- 前端源语言选择新增“非英语 Latin”检测：
  - 识别法语/西语常见词和重音字符。
  - 明显英语 UI 继续走 `en`，保持速度和稳定。
  - 明显法语/西语/其他 Latin 语言走 `auto`，交给供应商自动识别。
- 服务端新增同样的兜底：
  - `detect_source_lang_hint(text, "en")` 遇到明显非英语 Latin 时返回 `auto`。
  - Google batch 会按真实 source hint 分组，避免同一批中英语和非英语 Latin 互相污染。
- smoke 脚本同步路由策略：
  - 法语/西语现在进入 `source=auto` 批次。
  - 法语/西语不再只检查“有中文”，而是要求译文包含核心语义关键词：`打开`、`预览`、`保存`。
- 部署到 N100：
  - 使用 `deploy_n100_translation_server.ps1 -SkipPublicSmoke` 上传并重启服务。
  - 当前 N100 uvicorn pid：`2086881`。

### 新增文件

- 无。

### 修改文件

- `server/translator.py`
- `server/tests/test_translate_text.py`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不接入新的付费翻译通道。
- 不把所有 Latin 语言做成完整语言检测器；本章是低风险启发式路由修复。
- 不解决 OCR 对法语/西语截图的识别准确率，只验证文本翻译服务链路。

### 验证

- `npm run check:ocr-processing`：通过。
  - `Ouvrir l'aperçu avant d'enregistrer` 识别为非英语 Latin。
  - `Abrir vista previa antes de guardar` 识别为非英语 Latin。
  - 法语/西语目标中文时 `selectPreferredSourceLanguage` 返回 `auto`。
- `python -m pytest server\tests\test_translate_text.py`：通过，`7 passed`。
- `python -m pytest server\tests`：通过，`22 passed, 1 skipped`。
- `npm run build`：通过。
- `npm run check:i18n`：通过，`498 zh-CN keys match 498 en-US keys`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\deploy_n100_translation_server.ps1 -SkipPublicSmoke`：通过，N100 已部署。
- `npm run smoke:translate-service`：通过，走内网 N100。
  - `source=en`：8 blocks，client 约 `500ms`，server `464ms`，cache `3`，miss `5`。
  - `source=auto`：2 blocks，client 约 `155ms`，server `152ms`，miss `2`。
  - 法语：`Ouvrir l'aperçu avant d'enregistrer => 保存前打开预览`。
  - 西语：`Abrir vista previa antes de guardar => 保存前打开预览`。
  - 多行英文保持多行中文译文。
  - 技术文件名、命令/路径继续保留。

### 当前风险

- 非英语 Latin 检测是启发式，不是完整语言识别；未来如果要覆盖德语/葡语/意大利语/土耳其语等，需要扩大词表或引入轻量语言检测。
- Google 免费通道依旧不是商业级质量通道；本章修复了明确错误路由，但不代表所有多语言语义都可靠。
- 客户端和服务端短 UI 词典仍有重复维护问题，后续应抽成共享术语表或服务端返回 glossary version。

### 下一章建议

Chapter 121：建立翻译质量路由策略。优先做三件事：扩展 Latin 语言检测词表、为 Google 免费通道增加“低质量风险”标记，以及把短 UI 词典/术语表抽成可共享的 manifest，减少客户端和服务端重复维护。

## Chapter 121：共享翻译术语表 Manifest

### 目标

继续优化翻译系统，把 Chapter 116-120 形成的短 UI 词典从前端和服务端重复常量中抽离出来，收敛为一份可部署的术语表 manifest。后续扩展德语、葡语、意大利语、土耳其语等 Latin 语言检测和术语保护时，避免前端改一份、服务端漏一份。

### 本章实际处理

- 新增共享术语表：
  - `tauri-client/src/utils/translationGlossary.json`
  - 当前 version：`2026-06-03.1`
  - 包含 `zh.ui` 短 UI 术语，例如 `Save => 保存`、`Open preview => 打开预览`。
- 前端 `translationMemory.ts` 改为从 JSON manifest 构建本地短 UI 词典。
- 服务端 `translator.py` 改为加载同一份 manifest：
  - 本地开发时读取 `tauri-client/src/utils/translationGlossary.json`。
  - N100 部署后读取 server 目录内的 `translationGlossary.json`。
  - 如果 manifest 缺失，会 fallback 到空词典并记录风险，不阻断服务启动。
- 部署脚本更新：
  - `deploy_n100_translation_server.ps1` 会上传 `translationGlossary.json` 到 N100 server 目录。
  - 远端备份也包含 `translationGlossary.json`。
  - 重启等待逻辑补强，减少旧进程未释放端口时的竞态。
- 检查脚本新增 manifest 断言：
  - 前端检查 `Save => 保存`、`Open preview => 打开预览`。
  - 服务端测试确认不是 fallback 空词典。

### 新增文件

- `tauri-client/src/utils/translationGlossary.json`

### 修改文件

- `deploy_n100_translation_server.ps1`
- `server/translator.py`
- `server/tests/test_translate_text.py`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/src/utils/translationMemory.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不把术语表做成远程动态更新。
- 不加入完整语言检测模型。
- 不改变当前翻译通道配置；N100 仍使用当前 active channel。

### 验证

- `npm run check:ocr-processing`：通过。
- `python -m pytest server\tests\test_translate_text.py`：通过，`7 passed`。
- `python -m pytest server\tests`：通过，`22 passed, 1 skipped`。
- `npm run build`：通过。
- `npm run check:i18n`：通过，`498 zh-CN keys match 498 en-US keys`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\deploy_n100_translation_server.ps1 -SkipPublicSmoke`：通过。
  - 远端备份：`deploy-backups/20260603-005722`。
  - 上传 `app.py`、`translator.py`、`translationGlossary.json`。
  - LAN health 通过。
  - LAN timing smoke 通过。
- `npm run smoke:translate-service`：通过。
  - 法语/西语继续返回 `保存前打开预览`。
  - `Save => 保存` 由共享术语表保证。

### 当前风险

- manifest 目前只覆盖中文目标语言的高频 UI 短语，仍不是完整术语库。
- 服务端 fallback 到空词典时不会阻断启动；后续如果术语表变成关键资产，应让健康检查暴露 manifest version 和加载状态。
- 远端部署脚本目前可用，但仍不是 systemd/supervisor 级进程管理。

### 下一章建议

Chapter 122：让服务端 `/api/health` 或 `/api/config/current` 暴露 `translation_glossary_version` 和 translator quality flags；前端状态栏或设置页可以显示当前术语表版本，帮助定位“客户端和 N100 是否同步”。

## Chapter 122：翻译服务元数据与术语表版本可观测性

### 目标

Chapter 121 已把短 UI 词典抽成共享 `translationGlossary.json`，但用户和客户端仍无法从服务状态直接确认 N100 是否加载了同版本术语表。本章目标是让服务端暴露翻译运行时 metadata，并让客户端状态栏 tooltip 能显示关键同步信息。

### 本章实际处理

- 服务端新增翻译运行时 metadata：
  - `glossary_version`
  - `glossary_loaded`
  - `glossary_terms`
  - `quality_flags`
- `/api/health` 返回：
  - `translation.active_channel`
  - `translation.glossary_version`
  - `translation.glossary_loaded`
  - `translation.glossary_terms`
  - `translation.quality_flags`
- `/api/config/current` 带 token 返回同样的 `translation` metadata。
- 当前 quality flags：
  - `short_ui_glossary`
  - `latin_non_english_auto_source`
  - `multiline_block_preserved`
  - `technical_identifier_preservation`
  - `google_free_low_quality_risk`
- 前端 `useServerStatus` 解析 `/api/health` 的 `translation` metadata。
- 顶部服务状态 tooltip 增加：
  - 服务 URL
  - 当前 channel
  - glossary version
  - Google 免费通道多语言质量风险提示
- 部署脚本增强：
  - LAN health 后解析 `translation.glossary_loaded`。
  - 如果 N100 未加载术语表则部署失败。
  - 输出当前术语表版本和 term 数。

### 新增文件

- 无。

### 修改文件

- `deploy_n100_translation_server.ps1`
- `server/app.py`
- `server/tests/test_server.py`
- `server/translator.py`
- `tauri-client/src/App.tsx`
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/hooks/useServerStatus.ts`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/i18n/types.ts`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不新增显眼设置面板；只在顶部服务状态 tooltip 里显示精简 metadata。
- 不改变当前 active channel。
- 不解决 Google 免费通道的根本质量问题；只把风险显式暴露出来。

### 验证

- `python -m pytest server\tests`：通过，`24 passed, 1 skipped`。
- `npm run check:i18n`：通过，`501 zh-CN keys match 501 en-US keys`。
- `npm run build`：通过。
- `npm run check:ocr-processing`：通过。
- N100 `/api/health`：通过，返回：
  - `active_channel: google`
  - `glossary_version: 2026-06-03.1`
  - `glossary_loaded: true`
  - `glossary_terms: 24`
  - `google_free_low_quality_risk: true`
- N100 `/api/config/current`：通过，带 token 返回同样 translation metadata。
- `npm run smoke:translate-service`：通过，走内网 N100，13 blocks / 5 batches 约 `2081ms`。

### 当前风险

- tooltip 只显示精简状态；如果后续 flags 增多，设置页或诊断报告应提供完整 JSON。
- Google 免费通道风险已经可见，但还没有自动切换到更高质量通道。
- 服务端 metadata 现在来自进程内加载状态；如果未来术语表支持热更新，需要加 reload 或重启提示。

### 下一章建议

Chapter 123：建立翻译质量路由策略的用户可见诊断。把 `google_free_low_quality_risk`、术语表版本、服务端 channel、缓存命中等信息纳入诊断报告，并为 Google 通道提供“推荐切换到 Baidu/New API”的非阻断提示。

## Chapter 123：翻译系统诊断报告增强

### 目标

Chapter 122 已让 N100 服务端暴露翻译 metadata。本章把这些信息纳入“复制诊断报告”，让用户或开发者一键获得当前翻译服务 URL、channel、术语表版本、缓存状态和质量风险，而不是手动请求 `/api/health`。

### 本章实际处理

- `ConfigPageHeader` 的“复制诊断报告”增强：
  - 仍先读取 Tauri 原生 `get_diagnostics_report`。
  - 再读取本机配置，解析当前优先翻译服务地址：
    - 开启 LAN 优先时使用 `lanServerUrl`。
    - 否则使用 `serverUrl` / 默认公网服务。
  - 请求 `${serviceUrl}/api/health`。
  - 将翻译诊断合并到报告 `translation` 字段。
- 新增诊断内容：
  - `serviceUrl`
  - `configuredChannel`
  - `targetLang`
  - `localGlossaryVersion`
  - `localTranslationCache`
  - `serverHealth`
  - `serverTranslation`
  - `healthError`
  - `qualityWarnings`
- 当前 quality warnings：
  - `google-free-low-quality-risk`
  - `glossary-version-mismatch`
  - `server-glossary-not-loaded`

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/components/config/ConfigPageHeader.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不在界面新增大型诊断面板。
- 不自动切换翻译通道。
- 不修改 N100 服务端；Chapter 122 的服务端 metadata 已部署。

### 验证

- `npm run build`：通过。
- `npm run check:i18n`：通过，`501 zh-CN keys match 501 en-US keys`。
- `npm run check:ocr-processing`：通过。
- N100 `/api/health` 当前已返回：
  - `glossary_version: 2026-06-03.1`
  - `glossary_loaded: true`
  - `glossary_terms: 24`
  - `google_free_low_quality_risk: true`
- `npm run smoke:translate-service`：通过，13 blocks / 5 batches。

### 当前风险

- 诊断报告会包含服务 URL、channel 和缓存统计，但不会包含第三方密钥。
- Google 免费通道质量风险只作为 warning 暴露，还没有自动策略切换。
- 如果服务端 health 请求失败，诊断报告会记录 `healthError`，不会阻塞复制。

### 下一章建议

Chapter 124：翻译通道质量策略 UI。基于 `google_free_low_quality_risk` 给设置页或诊断卡片增加非阻断提示：Google 免费通道适合快速 smoke，不建议作为商业级多语言质量通道；推荐配置 Baidu 或 New API。

## Chapter 124：Google 免费通道质量风险提示

### 目标

Chapter 122-123 已让服务端和诊断报告暴露 `google_free_low_quality_risk`。本章把这个质量风险转成用户可见、非阻断的设置页提示：Google 免费通道可用于快速 smoke，但不应被误认为商业级多语言质量通道。

### 本章实际处理

- 设置页“翻译通道”卡片在当前通道为 `google` 时显示 warning。
- 提示内容：
  - Google 免费通道适合快速测试。
  - 多语言语义质量不稳定。
  - 正式使用建议配置百度或大模型翻译。
- 提示不阻断保存、不改变默认通道、不自动切换通道。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/i18n/dictionaries.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不自动切换到百度或 New API。
- 不新增复杂通道评分系统。
- 不改变服务端 active channel。

### 验证

- `npm run check:i18n`：通过，`503 zh-CN keys match 503 en-US keys`。
- `npm run build`：通过。
- `npm run check:ocr-processing`：通过。

### 当前风险

- 只是提示风险，真正商业级质量仍需要接入和验证更可靠的付费/LLM 通道。
- 目前提示基于前端当前选择的 channel；如果服务端 active channel 与客户端表单未同步，仍需依靠 Chapter 123 诊断报告定位。

### 下一章建议

Chapter 125：继续翻译通道质量策略。优先把百度/New API 的“已配置/未配置/测试通过”状态做成更清楚的通道健康摘要，然后再回到 OCR 固定 crop fixture。

## Chapter 125：翻译通道健康摘要与内网优先设置同步

### 目标

Chapter 124 已提示 Google 免费通道的质量风险，但设置页仍不能直接看出百度/New API 是否配置完整、是否测试通过，以及保存/测试到底命中了内网 N100 还是公网服务。本章目标是把翻译通道的可用性做成用户可见摘要，并修正设置侧的内网优先同步路径。

### 本章实际处理

- 设置页“翻译通道”卡片新增通道健康摘要：
  - Google 显示无需密钥、当前通道和质量风险。
  - 百度显示 App ID / Secret Key 配置完整性、最近测试状态。
  - New API 显示中转地址 / API Key / Model 配置完整性、最近测试状态。
  - 显示服务端当前通道和实际同步 URL。
- `useSettingsController` 新增状态：
  - `channelTestStatuses`
  - `serverChannelStatus`
- 设置侧请求改为内网优先候选 URL：
  - 加载设置时同步 `/api/config/current` 先试 `lanServerUrl`，再回落 `serverUrl`。
  - `fetchModels` 先试内网 N100，再回落公网。
  - `testChannel` 先试内网 N100，再回落公网，并记录测试通过/失败状态。
  - `saveServerChannelConfig` 先试内网 N100，再回落公网，并记录实际保存 URL。
- `main.tsx` 增加普通浏览器 dev fallback：
  - Tauri 环境照旧读取真实窗口 label。
  - 普通浏览器缺少 `__TAURI_INTERNALS__` 时使用 `main`，避免 Vite 页面白屏，方便后续前端自动化验收。

### 新增文件

- 无。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/src/main.tsx`
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/components/settings/types.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/pages/Settings.tsx`

### 删除文件

- 无。

### 本章不做

- 不自动切换到百度或 New API。
- 不保存第三方密钥测试结果到诊断报告。
- 不改变 N100 当前 active channel；当前仍为 `google`。
- 不把 Google 免费通道包装成商业级质量通道。

### 验证

- `npm run check:i18n`：通过，`520 zh-CN keys match 520 en-US keys`。
- `npm run build`：通过。
- `npm run check:ocr-processing`：通过。
- 普通浏览器 dev 页面：通过。
  - `main.tsx` fallback 后 `http://127.0.0.1:1420/` 可渲染设置页。
  - “通道健康摘要”显示 Google/Baidu/New API 配置状态、测试状态和服务端同步状态。
- N100 LAN `/api/health`：通过。
  - `active_channel: google`
  - `glossary_version: 2026-06-03.1`
  - `glossary_loaded: true`
  - `glossary_terms: 24`
  - `google_free_low_quality_risk: true`
- `npm run smoke:translate-service`：通过。
  - 走 `http://192.168.1.3:8318`。
  - 13 blocks / 5 batches。
  - 本次缓存命中后总耗时约 `57ms`。

### 当前风险

- 当前通道仍为 Google 免费通道；健康摘要只降低误用风险，不提升 Google 本身质量。
- 浏览器 dev fallback 只用于本地前端验收；真实 Tauri 窗口仍以 Tauri metadata 为准。
- 百度/New API 的测试结果只保存在当前设置页会话中，重启后需要重新测试才能显示“测试通过”。

### 下一章建议

Chapter 126：回到 OCR 固定 crop fixture。建立英文 UI、中文大段、小字技术文本三类真实图片样例门禁，把当前一次性的 OCR 截图测试变成可重复的本地 smoke，并输出识别文本与耗时。

## Chapter 126：固定 OCR Crop Fixture 门禁

### 目标

前面多章一直把“回到 OCR 固定 crop fixture”作为下一步，但实际只有 `YSN_OCR_SMOKE_IMAGE` 这种一次性环境变量入口。本章目标是建立可重复的本地 OCR fixture 门禁，覆盖英文 UI、中文大字、小字技术文本三类样例，并把当前 OCR 输出和耗时固定成后续优化基线。

### 本章实际处理

- Rust ignored OCR smoke 增强：
  - 保留 `smoke_local_ocr_image_from_env` 单图入口。
  - 新增 `smoke_local_ocr_fixtures_from_env`，读取 `YSN_OCR_SMOKE_FIXTURES` JSON 数组。
  - 每个 fixture 支持：
    - `name`
    - `image_path`
    - `expect_contains`
    - `min_blocks`
    - `known_issues`
  - OCR 输出会打印 blocks、耗时和已知问题。
  - 关键词断言会做大小写和空白归一化。
- 新增 `tauri-client/scripts/check-ocr-fixtures.ps1`：
  - 在临时目录生成三张 PNG，不把二进制图片放进仓库。
  - 使用 Windows `System.Drawing` 绘制固定文本。
  - 中文文本用 Unicode code point 拼接，避免 Windows PowerShell 5.1 按 ANSI 读取 `.ps1` 导致 fixture 本身乱码。
  - 一次性设置 `YSN_OCR_MODEL_ROOT` 和 `YSN_OCR_SMOKE_FIXTURES`，运行真实 Rust OCR pipeline。
- `package.json` 新增：
  - `npm run check:ocr-fixtures`
- 根目录商业检查新增可选开关：
  - `.\check_commercial.ps1 -OcrFixtures`
  - 默认不跑真实 OCR fixture，避免无模型环境被阻断。

### 当前固定 OCR 基线

- `chinese-large`：
  - 输出 `保存前打开预览`
  - 输出 `复制翻译文本`
  - 基本准确，耗时约 `2.3s-2.5s`。
- `english-ui`：
  - 输出 `Open preview before savin`
  - 输出 `Copy translated tex`
  - 核心语义正确，但会丢英文末尾字符。
- `technical-small`：
  - 输出 `PATH=C: \Windows\System3:`
  - 输出 `LocalModel.exe --hel]`
  - 能保留 PATH / Windows / LocalModel.exe 核心标识，但小字尾部数字和命令尾部仍会误读。

### 新增文件

- `tauri-client/scripts/check-ocr-fixtures.ps1`

### 修改文件

- `check_commercial.ps1`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/package.json`
- `tauri-client/src-tauri/src/lib.rs`

### 删除文件

- 无。

### 本章不做

- 不把这些 fixture 当成 OCR production ready 证明。
- 不把 `runtimeInferenceReady` 改成 `true`。
- 不直接修复英文末尾截断或技术小字尾部误读；本章先建立稳定门禁和基线。
- 不把生成的 PNG fixture 写进仓库。

### 验证

- `npm run check:ocr-fixtures`：通过。
  - 三个 fixture 共 6 个 blocks。
  - 中文大字准确。
  - 英文 UI 和小字技术文本暴露已知尾部误读。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1 -OcrFixtures`：通过。
  - `npm run check:i18n`：通过，`520 zh-CN keys match 520 en-US keys`。
  - `npm run check:ocr-processing`：通过。
  - `npm run build`：通过。
  - `cargo check`：通过。
  - `cargo test`：通过，`105 passed; 2 ignored; 0 failed`。
  - `npm run check:ocr-fixtures`：通过。

### 当前风险

- 固定 fixture 当前是生成式图片，不等于真实应用截图 crop；下一步仍要加入真实截图样例。
- 英文 UI 和技术小字仍有尾部字符误读，后续要做异常评分、输入尺寸/crop 策略或 fallback。
- `System.Drawing` fixture 生成脚本面向 Windows，本项目当前产品目标也是 Windows 桌面应用。

### 下一章建议

Chapter 127：基于固定 OCR fixture 增加 Latin 技术文本异常评分。优先捕捉英文 UI 末尾截断、`System32` 数字尾部误读、`--help` 命令尾部误读，并把评分结果输出到 smoke，为 fallback 或二次识别做准备。

## Chapter 127：Latin OCR 异常评分与 Smoke 输出

### 目标

Chapter 126 已建立固定 OCR fixture，并暴露出两个稳定问题：英文 UI 文案末尾截断、小字技术文本尾部误读。本章目标是不急着做高风险自动修复，而是先把这些错误变成结构化 quality issue，进入真实 OCR 日志和 fixture 门禁。

### 本章实际处理

- 新增 `ysn_ocr_quality` 模块：
  - `score_latin_ocr_text_anomalies`
  - `OcrTextQualityIssue`
- 当前 issue code：
  - `latin-probable-tail-truncation`
  - `technical-path-extra-space`
  - `technical-path-digit-tail`
  - `technical-command-flag-tail`
  - `latin-unmatched-bracket-tail`
- 评分规则当前只处理含 ASCII 字母的 Latin/技术文本：
  - 不误伤纯中文 fixture。
  - 不误伤正常英文 `Open preview before saving` / `Copy translated text`。
  - 能抓到当前 fixture 的 `savin`、`tex`、`System3:`、`--hel]`。
- 真实 OCR pipeline 在发现 issue 时输出：
  - `[local-screenshot-translate] ocr quality issues [...]`
- 固定 OCR fixture JSON 增加：
  - `expect_quality_issues`
  - smoke 会断言指定 issue code 被捕捉到。

### 新增文件

- `tauri-client/src-tauri/src/ysn_ocr_quality.rs`

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `tauri-client/src-tauri/src/lib.rs`

### 删除文件

- 无。

### 本章不做

- 不根据评分自动修改 OCR 文本。
- 不触发二次识别或 fallback。
- 不把 quality issue 暴露到前端 UI；当前先进入日志和 smoke。
- 不把 `runtimeInferenceReady` 改成 `true`。

### 验证

- `cargo test ysn_ocr_quality --lib`：通过，`4 passed`。
- `npm run check:ocr-fixtures`：通过。
  - 中文 fixture：`quality issues: []`。
  - 英文 UI fixture：捕捉 `latin-probable-tail-truncation`。
  - 技术小字 fixture：捕捉 `technical-path-extra-space`、`technical-path-digit-tail`、`technical-command-flag-tail`，并额外输出 bracket/tail truncation 信号。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1 -OcrFixtures`：通过。
  - `npm run check:i18n`：通过，`520 zh-CN keys match 520 en-US keys`。
  - `npm run check:ocr-processing`：通过。
  - `npm run build`：通过。
  - `cargo check`：通过。
  - `cargo test`：通过，`109 passed; 2 ignored; 0 failed`。
  - `npm run check:ocr-fixtures`：通过。

### 当前风险

- 评分仍是启发式，不等同完整 OCR 质量模型。
- 当前只记录和断言 issue，不会自动恢复错误文本。
- 如果后续扩大规则，必须继续保证不误伤中文、正常英文 UI 和受保护技术标识。

### 下一章建议

Chapter 128：把 OCR quality issues 接入低风险 fallback。优先只对 Latin 技术文本尝试更宽 crop / 更大 recognition width 的二次识别；若二次识别仍不改善，则把 quality warning 暴露到调试结果，而不是静默当成高质量 OCR。

## Chapter 128：Latin OCR 质量重试与技术路径修复

### 目标

Chapter 127 已经能发现英文 UI 末尾截断和小字技术文本尾部误读。本章目标是把这些 quality issue 接入低风险修复路径：只对命中异常的 Latin 行做二次识别，不拖慢中文和正常文本；能改善就替换，不能改善就保留原结果和日志。

### 本章实际处理

- Latin 初次识别后新增 quality retry：
  - 命中 `latin-probable-tail-truncation` 或 warning issue 时触发。
  - 二次识别尝试更大的 recognition width：`480`、`640`。
  - 同时尝试对原 crop 右侧和少量边缘扩展后再识别。
  - 只有 candidate 的 quality penalty 更低、token 更多、置信度没有明显下降时才替换。
- 新增 retry 统计日志：
  - `[local-screenshot-translate] ocr latin quality retries attempts=... improvements=...`
- 新增窄范围技术路径空格修复：
  - 只处理看起来像 Windows/path 的 Latin 技术文本。
  - 修复 `C: \Windows \System32` 为 `C:\Windows\System32`。
  - 不处理普通英文句子。
- 固定 OCR fixture 预期升级：
  - 英文 UI 现在要求完整 `Open preview before saving` 和 `Copy translated text`。
  - 技术小字现在要求完整 `PATH=C:\Windows\System32` 和 `LocalModel.exe --help`。
  - 英文和技术 fixture 的 `quality issues` 均要求为空。

### 修复效果

- `english-ui` 修复前：
  - `Open preview before savin`
  - `Copy translated tex`
- `english-ui` 修复后：
  - `Open preview before saving`
  - `Copy translated text`
  - `quality issues: []`
- `technical-small` 修复前：
  - `PATH=C: \Windows\System3:`
  - `LocalModel.exe --hel]`
- `technical-small` 修复后：
  - `PATH=C:\Windows\System32`
  - `LocalModel.exe --help`
  - `quality issues: []`

### 新增文件

- 无。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/ysn_ocr_quality.rs`

### 删除文件

- 无。

### 本章不做

- 不把所有 Latin 文本都强制二次识别。
- 不改变中文/CJK 正常识别路径。
- 不把 `runtimeInferenceReady` 改成 `true`。
- 不把启发式评分包装成完整 OCR 质量模型。

### 验证

- `cargo test ysn_ocr_quality --lib`：通过，`5 passed`。
- `cargo check`：通过。
- `npm run check:ocr-fixtures`：通过。
  - 中文 fixture 仍准确。
  - 英文 UI fixture 输出完整英文，quality issues 为空。
  - 技术小字 fixture 输出完整路径和命令，quality issues 为空。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1 -OcrFixtures`：通过。
  - `npm run check:i18n`：通过，`520 zh-CN keys match 520 en-US keys`。
  - `npm run check:ocr-processing`：通过。
  - `npm run build`：通过。
  - `cargo check`：通过。
  - `cargo test`：通过，`110 passed; 2 ignored; 0 failed`。
  - `npm run check:ocr-fixtures`：通过。

### 当前风险

- 当前修复只在生成式固定 fixture 上验证，还需要真实截图 crop 验证。
- 二次识别增加了命中异常行的 recognition 时间；但只对异常 Latin 行触发，中文和无异常文本不走该路径。
- 技术路径空格修复是窄范围规则，后续遇到 URL、Linux path、复杂命令时还需要扩展测试。

### 下一章建议

Chapter 129：扩展固定 OCR fixture 到真实截图 crop。优先用应用设置页、英文 UI、路径/命令区域生成真实截图样例，比较生成式 fixture 与真实 crop 的 OCR 输出、quality retry 命中率和耗时。

## Chapter 129：真实浏览器截图 OCR Fixture

### 目标

Chapter 128 已经让生成式英文 UI 和技术小字 fixture 通过更宽 crop / 更大 recognition width 的低风险二次识别恢复完整文本。本章目标是把门禁从“代码生成图片”推进到“真实应用截图”，确认真实渲染、字体抗锯齿、页面布局和中英混排下同一条 OCR pipeline 仍能跑通。

### 本章实际处理

- 使用本机 Vite 应用真实页面生成截图：
  - 页面地址：`http://127.0.0.1:1420/`
  - 截图文件：`C:\Users\ysn\AppData\Local\Temp\ysn-real-browser-shot.png`
  - 截图方式：Codex Browser 打开应用页面并保存 viewport PNG。
- 扩展 `check-ocr-fixtures.ps1`：
  - 新增 `-RealScreenshotPath`。
  - 新增 `-RealExpectContains`。
  - 新增 `-RealMinBlocks`。
  - 真实截图 fixture 由调用方提供，不写入仓库，避免把临时屏幕截图当成长期资产。
- 真实截图 fixture 与生成式 fixture 走同一套 Rust ignored test：
  - `smoke_local_ocr_fixtures_from_env`
  - 同样使用项目 `models/ocr` active 模型。
  - 同样断言期望文本、最少 block 数和 quality issues。

### 真实截图结果

- 真实截图识别通过：
  - 最少 block 断言：`12 blocks >= 8`。
  - 关键文本断言：包含 `YSN`、`OCR`。
  - quality issues：`[]`。
- 真实截图样例输出中还能看到：
  - `全局快捷键注册失败`
  - `系统设置`
  - 部分小 UI / 图标附近仍有噪声，如 `AItA`、`©`、`/`。
- 真实截图耗时样例：
  - 总耗时约 `2234ms`。
  - detector 约 `1028ms`。
  - crop 约 `98ms`。
  - recognizer 约 `467ms`。
  - detections/crops：`12`。
  - CJK fallbacks：`5`。

### 新增文件

- 无。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/scripts/check-ocr-fixtures.ps1`

### 删除文件

- 无。

### 本章不做

- 不把临时真实截图 PNG 提交到仓库。
- 不把真实截图 fixture 默认塞进 `npm run check:ocr-fixtures`，因为该图片依赖本机当前 UI 状态；当前先作为可选参数门禁。
- 不把 `runtimeInferenceReady` 改成 `true`。
- 不处理真实截图里的全部噪声 block；本章先建立可复跑入口和基线数据。

### 验证

- `powershell -NoProfile -ExecutionPolicy Bypass -Command "& '.\scripts\check-ocr-fixtures.ps1' -RealScreenshotPath 'C:\Users\ysn\AppData\Local\Temp\ysn-real-browser-shot.png' -RealExpectContains 'YSN','OCR' -RealMinBlocks 8"`：通过。
  - `chinese-large`：通过。
  - `english-ui`：通过。
  - `technical-small`：通过。
  - `real-screenshot`：通过，`12 blocks`，包含 `YSN`、`OCR`。

### 当前风险

- 当前真实截图只覆盖一个本机浏览器页面，不等于完整 `Ctrl+D` 透明 overlay 人工验收。
- 真实截图中仍有 UI 图标/小字噪声 block，后续需要 detector 后处理、文本块过滤或质量 scoring。
- detector 仍是最大耗时来源，真实截图样例里约 `1.0s`。

### 下一章建议

Chapter 130：开始压低 detector 耗时。优先评估 detector 输入缩放、白底 UI 快速路径、最小文本区域过滤和结果复用；所有改动必须继续通过生成式 fixture、真实截图 fixture、`cargo check` 和商业检查中的 OCR fixture。

## Chapter 130：翻译同轮去重与 N100 部署复测

### 目标

用户明确要求“快狠准”。本章先处理翻译链路里风险最低、收益直接的速度点：同一次 OCR/翻译请求中，如果多个 block 文本完全相同，不应重复发给翻译 provider。前端先去重减少网络 payload，服务端再兜底，保证旧客户端或其他调用方也不会重复打 Google/Baidu/LLM。

### 本章实际处理

- 前端 `translateWithLocalOcr` 增加请求级去重：
  - 本机缓存/术语表未命中的 block 先按规整文本聚合。
  - 相同文本只发给 `/api/translate_text` 一次。
  - 返回译文后回填所有原始 block index。
  - `translationMemoryStats` 新增 `deduplicatedBlocks`，保留本轮减少的重复 block 数。
- 服务端 `BaseTranslator.translate_batch` 增加同轮 miss 去重：
  - 术语表和全局缓存命中逻辑不变。
  - 全局缓存未命中的重复文本只进入一次 `_do_translate_batch`。
  - provider 返回后回填所有重复位置。
  - 服务端 timing 新增 `request_duplicates`。
  - `provider_misses` 改为真实 provider miss 数，而不是简单 `blocks - cache_hits`。
- smoke 增加动态重复文本样例：
  - `duplicate-probe-a`
  - `duplicate-probe-b`
  - 两者每次运行文本相同但 timestamp 不同，避免被历史缓存遮住本轮去重。
- 已部署到 N100：
  - 上传 `app.py`、`translator.py`、`translationGlossary.json`。
  - 远端 `py_compile` 通过。
  - 重启 `uvicorn app:app --host 0.0.0.0 --port 8318`。
  - LAN `/api/health` 正常。

### 实测结果

- 部署后首次完整 smoke：
  - `source=en blocks=10`
  - `cache=3`
  - `miss=6`
  - `dup=1`
  - 说明 10 个英文 block 中有 3 个缓存/术语表命中，6 个真实 provider miss，1 个重复文本被同轮去重。
  - 15 blocks / 5 batches 总计约 `1117ms`。
- 第二次热缓存 smoke：
  - `source=en blocks=10`
  - `cache=8`
  - `miss=1`
  - `dup=1`
  - 其它韩文、阿拉伯文、日文、法语/西语等样例命中缓存或通过质量断言。
  - 15 blocks / 5 batches 总计约 `758ms`。

### 新增文件

- 无。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `server/app.py`
- `server/translator.py`
- `server/tests/test_translator.py`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/src/utils/translationMemory.ts`

### 删除文件

- 无。

### 本章不做

- 不更换当前翻译通道；N100 当前仍是 `google`。
- 不把 Google 免费通道包装成商业级最终质量方案。
- 不改变 OCR 识别模型或 detector。
- 不把 `runtimeInferenceReady` 改成 `true`。

### 验证

- `python -m pytest server\tests\test_translator.py server\tests\test_translate_text.py`：通过，`13 passed, 1 skipped`。
- `npm run build`：通过。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`520 zh-CN keys match 520 en-US keys`。
- `npm run smoke:translate-service`：部署前通过，旧 N100 无 `dup` timing。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\deploy_n100_translation_server.ps1 -SkipPublicSmoke`：通过，N100 已部署。
- `npm run smoke:translate-service`：部署后通过，显示 `dup=1`。
- `python -m pytest server\tests`：通过，`25 passed, 1 skipped`。

### 当前风险

- 去重只对同一次请求里的相同规整文本生效；近似重复、OCR 轻微误差、大小写差异仍可能进入 provider。
- Google 免费通道的多语言质量风险仍存在；当前只是减少重复调用和提升速度。
- smoke 为了验证重复去重，每次动态生成 timestamp 文本，因此英文批次仍会保留 1 个真实 miss。

### 下一章建议

Chapter 131：继续“快狠准”主线。若继续翻译系统，优先做更可靠的付费/LLM 通道验收和 glossary/translation memory 的质量回滚；若回到 OCR，优先压 detector `~1s` 瓶颈并把真实截图噪声 block 过滤掉。

## Chapter 131：服务端技术文本保护与日文误伤修复

### 目标

Chapter 130 已经减少同轮重复文本 provider 调用。本章继续优化“快”和“准”：技术标识、路径、命令、文件名、flag 等文本不应依赖前端保护，也不应发送到 Google/Baidu/LLM 后祈祷它们不被改坏。服务端必须自己兜底保护，并暴露本轮保护数量。

### 本章实际处理

- 服务端新增技术文本保护规则：
  - 文件名/扩展名：如 `COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`。
  - Windows 路径和命令：如 `PATH=C:\Windows\System32 && LocalModel.exe --help`。
  - command flag、env assignment、package/path-like token、全大写技术标识。
  - 目标为中文时的纯中文文本，直接保留，不进入 provider。
- `BaseTranslator.translate_batch` 新增保护路径：
  - 在术语表、全局缓存、provider miss 之前判断。
  - 命中保护时直接回填原文。
  - `stats_ref["preserved_hits"]` 记录保护数量。
- `/api/translate_text` 的 `timings` 新增：
  - `preserved_hits`
- `smoke-translate-service.mjs` 显示：
  - `keep=<preserved_hits>`
- 修复一次真实 smoke 抓到的 bug：
  - 初版“中文目标下含汉字且无 Latin 即保留”的规则太宽。
  - 日文 `保存する前にプレビューを開く` 包含汉字和假名，被误判为纯中文。
  - 已收窄为：只有没有假名、韩文、阿语、泰语、俄文等其他可翻译脚本时，才按纯中文保留。

### 实测结果

- 修复前 smoke 失败：
  - `source=ja keep=1`
  - 日文原文被保留，导致 `japanese` 未翻译。
- 修复后 smoke 通过：
  - `source=ja miss=1 keep=0`
  - 日文翻译为 `保存前打开预览`。
  - 英文批次显示 `keep=2`，对应文件名和命令保护。
- 热缓存 smoke：
  - 15 blocks / 5 batches 约 `242ms`。
  - 英文批次：`cache=6 miss=1 dup=1 keep=2`。
  - 韩文、阿拉伯文、日文、法语、西语、多行英文、技术文本全部通过。

### 新增文件

- 无。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `server/app.py`
- `server/translator.py`
- `server/tests/test_translator.py`
- `tauri-client/scripts/smoke-translate-service.mjs`
- `tauri-client/src/utils/localOcrTranslate.ts`

### 删除文件

- 无。

### 本章不做

- 不引入新的 provider。
- 不把 Google 免费通道视为最终商业质量方案。
- 不改变前端已有技术文本保护；本章是服务端兜底。
- 不把 `runtimeInferenceReady` 改成 `true`。

### 验证

- `python -m pytest server\tests\test_translator.py server\tests\test_translate_text.py`：通过，`15 passed, 1 skipped`。
- `python -m pytest server\tests`：通过，`27 passed, 1 skipped`。
- `npm run build`：通过。
- `npm run check:ocr-processing`：通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\deploy_n100_translation_server.ps1 -SkipPublicSmoke`：通过，N100 已部署。
- `npm run smoke:translate-service`：修复后通过。
- `npm run smoke:translate-service`：热缓存复跑通过，约 `242ms`。

### 当前风险

- 技术文本保护仍是启发式规则，后续遇到 URL、Linux path、复杂 shell pipeline、代码片段时还需要继续扩充测试。
- 纯中文保留规则已避免日文误伤，但 CJK 混排仍需要真实样例持续覆盖。
- Google 免费通道仍有质量风险；当前优化主要降低重复调用和技术文本误翻。

### 下一章建议

Chapter 132：如果继续翻译系统，优先做“质量可回滚”的 translation memory：低质量/错译条目可按服务端版本、provider、source hash 失效；如果回到 OCR，继续压 detector `~1s` 瓶颈并过滤真实截图噪声 block。

## Chapter 132：截图翻译覆盖层原位原字号渲染

### 目标

用户在真实构建中发现翻译覆盖结果“字体非常小、段落不整齐、位置乱动”。本章目标是修复渲染策略：译文必须跟随原文 OCR 框，原文在哪里译文就在哪里；字号必须接近原文，不能为了塞进框里无限缩小。

### 本章实际处理

- 停用渲染层的相邻 OCR block 合并：
  - 旧逻辑会把同一行/邻近行合并成 group，再统一擦除和重画。
  - 新逻辑保持 OCR block 一对一渲染，避免段落被重排。
- 改为原位锚定：
  - LTR 文本从原 OCR block `minX` 开始画。
  - RTL 文本从原 OCR block `maxX` 开始画。
  - 单行译文垂直居中在原 OCR block 高度内。
- 修复字号策略：
  - 旧逻辑使用 `fitText`，会为了塞进高度/宽度持续降字号，最低到 7/8px。
  - 新逻辑从原 OCR 框高度估算原字号，不再因为译文稍长就压成极小字。
  - 背景擦除范围可以按译文宽度轻微延伸，但文本起点不变。
- 保留显式换行：
  - 行内多空格仍会压缩。
  - `\n` 不再被全部压成空格，避免多行翻译失去段落结构。
- 更新处理链路门禁：
  - `check-ocr-processing` 新增断言：两个相邻 OCR block 必须保持两个 render block，并保持各自原始 `minX/minY`。

### 视觉验证

- 临时创建本地 `render-check.html`，通过 Vite 正常加载当前 `renderTranslatedBlocks` 源码模块。
- 生成包含两行英文的 canvas：
  - `How are you?`
  - `Oh hi, Ben.`
- 渲染译文：
  - `你好吗？`
  - `哦，Ben。`
- 浏览器截图确认：
  - 两行译文没有合并。
  - 起点仍锁在原始 OCR 框。
  - 字号接近原文，不再出现极小字。
- 临时验证页已删除，未作为长期文件保留。

### 新增文件

- 无。

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/src/translation-render/renderBlockLayout.ts`
- `tauri-client/src/translation-render/renderTranslatedBlocks.ts`

### 删除文件

- 无。

### 本章不做

- 不改 OCR 识别结果。
- 不改翻译 provider。
- 不新增截图翻译设置项；当前先把默认行为改到正确。
- 不把 `runtimeInferenceReady` 改成 `true`。

### 验证

- `npm run check:ocr-processing`：通过。
- `npm run build`：通过。
- 本地浏览器 canvas 视觉回归：通过。
- 临时验证页清理确认：`tauri-client/render-check.html` 不存在。

### 当前风险

- 原位原字号渲染会优先保证准确位置和可读字号；如果译文极长，可能向右延伸或被画布边界截断，后续需要更精细的“同字号换行且不乱位”策略。
- 当前字体估算来自 OCR 框高度，若 OCR 框本身明显偏小/偏大，仍可能有偏差；后续可加入源图像文字高度采样或用户可调倍率。
- 背景擦除仍是取 block 周围颜色估算，复杂背景下可能需要更强的 inpaint/局部背景恢复。

### 下一章建议

Chapter 133：继续截图翻译渲染验收。优先用用户提供的真实页面样例复测，补“同字号换行但不重排”的长句策略，并把 render visual fixture 固化成可重复脚本，而不是临时浏览器页面。

## Chapter 133：RapidOCR 主路径迁移与配置面板闭环

### 目标

用户明确决定停止继续自研 `YSN OCR Runtime`，默认主路径改为 RapidOCR，并要求配置面板也做好。本章目标是一次性完成 OCR 架构切换：产品主流程走内置 RapidOCR runner，普通用户看到清晰的 RapidOCR 状态和自测入口，旧自研 Rust OCR 模块不再作为主路径保留。

### 本章实际处理

- 新增 RapidOCR Python runner：
  - 使用 `rapidocr==3.8.1` + ONNXRuntime。
  - 默认 PP-OCRv5，支持 PP-OCRv4。
  - 支持 `auto/full/latin` 模式、`--probe`、`--warm-models` 和 JSON 输出。
  - 候选包含 `ch/latin/korean/arabic/cyrillic/th`。
  - 增加脚本字符评分、低内容单块噪声扣分、阿拉伯文/韩文等脚本命中奖励，避免一个高置信假阳性 block 赢过真实多行候选。
  - 增加后处理：英文重叠碎片去重、技术路径空格清理、日文长音碎片修正、低置信 tiny noise 过滤。
- Rust OCR 主流程改为调用 RapidOCR runner：
  - `run_local_ocr` 写入临时 PNG 后调用 `rapidocr-runner`。
  - 新增 `get_rapid_ocr_status`、`run_rapid_ocr_self_test`。
  - 兼容开发态 Python script 和生产态 resource runner。
  - 删除旧自研 `ysn_ocr_*` Rust 模块注册和主路径依赖。
- 配置中心改为 RapidOCR 面板：
  - 显示 runner 是否就绪、模型版本、runner 路径/类型、模型目录、probe 耗时。
  - 支持刷新和自测。
  - 默认本地 OCR 开启，远端 OCR fallback 关闭。
  - 默认模型版本为 PP-OCRv5，高级可选 PP-OCRv4。
- 打包链路：
  - 新增 RapidOCR requirements。
  - 新增 `build-rapidocr-runner.ps1`，使用隔离 venv 和 PyInstaller onedir 打包。
  - 预热 V5/V4 多语言模型，避免用户首次使用时临时下载。
  - 清理旧 onefile 遗留 `rapidocr-runner.exe`，资源目录从约 `3.1 GB` 降到约 `340.7 MB`。
  - Tauri resource 纳入 `resources/rapidocr/**/*`。
- 测试夹具：
  - 新增 `check-ocr-fixtures.ps1`。
  - 生成并校验中文大字、英文 UI、小字技术文本、韩文、日文、阿拉伯文。
  - 开发版 runner 和打包版 runner 都跑同一套夹具。
- 文档方向同步：
  - `AGENTS.md` 和主计划从旧 `YSN OCR Runtime` 主线改为 RapidOCR / ONNXRuntime 主线。

### 新增文件

- `tauri-client/scripts/build-rapidocr-runner.ps1`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `tauri-client/src-tauri/rapidocr/rapidocr_runner.py`
- `tauri-client/src-tauri/rapidocr/requirements.txt`
- `tauri-client/src-tauri/resources/rapidocr/.gitkeep`
- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/rapidocr-runner.exe`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/hooks/useRapidOcrController.ts`

### 修改文件

- `AGENTS.md`
- `.gitattributes`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/package.json`
- `tauri-client/src-tauri/Cargo.toml`
- `tauri-client/src-tauri/Cargo.lock`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src/components/config/ConfigReadinessOverview.tsx`
- `tauri-client/src/components/config/ConfigRecoveryChecklist.tsx`
- `tauri-client/src/hooks/useOcrConfigController.tsx`
- `tauri-client/src/ocr-models/index.ts`
- `tauri-client/src/ocr-models/types.ts`
- `tauri-client/src/pages/About.tsx`
- `tauri-client/src/pages/OcrConfig.tsx`
- `tauri-client/src/ocr-processing/translationPolicy.ts`
- `tauri-client/src/utils/ocrConfigHelpers.ts`

### 删除文件

- `scripts/install_ppocrv5_onnx_models.ps1`
- `tauri-client/src-tauri/src/ysn_ocr_crop.rs`
- `tauri-client/src-tauri/src/ysn_ocr_decode.rs`
- `tauri-client/src-tauri/src/ysn_ocr_dictionary.rs`
- `tauri-client/src-tauri/src/ysn_ocr_manifest_store.rs`
- `tauri-client/src-tauri/src/ysn_ocr_model_downloader.rs`
- `tauri-client/src-tauri/src/ysn_ocr_model_index.rs`
- `tauri-client/src-tauri/src/ysn_ocr_model_schema.rs`
- `tauri-client/src-tauri/src/ysn_ocr_model_sources.rs`
- `tauri-client/src-tauri/src/ysn_ocr_pipeline.rs`
- `tauri-client/src-tauri/src/ysn_ocr_postprocess.rs`
- `tauri-client/src-tauri/src/ysn_ocr_preprocess.rs`
- `tauri-client/src-tauri/src/ysn_ocr_quality.rs`
- `tauri-client/src-tauri/src/ysn_ocr_router.rs`
- `tauri-client/src-tauri/src/ysn_ocr_runtime.rs`
- `tauri-client/src-tauri/src/ysn_ocr_runtime_adapter.rs`
- `tauri-client/src/components/config/ActiveModelHealthPanel.tsx`
- `tauri-client/src/components/config/ManagedSourceDryRunResultAlert.tsx`
- `tauri-client/src/components/config/ManagedSourceImportResultAlert.tsx`
- `tauri-client/src/components/config/ModelPackOperationStatus.tsx`
- `tauri-client/src/components/config/ModelPackStatusList.tsx`
- `tauri-client/src/components/config/ModelSourceStageGuide.tsx`
- `tauri-client/src/components/config/OcrModelPackPanel.tsx`
- `tauri-client/src/components/config/OcrRuntimeReadinessSteps.tsx`
- `tauri-client/src/hooks/useYsnOcrRuntimeController.ts`
- `tauri-client/src/ocr-models/manifest.ts`
- `tauri-client/src/ocr-models/modelPackService.ts`
- `tauri-client/src/ocr-models/operations.ts`

### 本章不做

- 不恢复旧自研 `YSN OCR Runtime` 主路径。
- 不恢复 PaddleOCR-json 作为普通用户 OCR 主路径。
- 不把 RapidOCR 识别质量称为最终完成；真实截图和复杂背景还要继续验收。
- 不提交、不推送。

### 验证

- `npm run check:ocr-fixtures`：通过。
  - 中文大字、英文 UI、小字技术文本、韩文、日文、阿拉伯文均通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\tauri-client\scripts\check-ocr-fixtures.ps1 -RunnerPath .\tauri-client\src-tauri\resources\rapidocr\rapidocr-runner\rapidocr-runner.exe`：通过。
  - 打包版 runner 夹具均通过。
  - 常见中英/日文快速路径约 `1.3–2.1s`。
  - 韩文/阿拉伯文完整 fallback 约 `5.6–6.3s`。
- `npm run build:rapidocr-runner -- -SkipInstall`：通过。
  - V5/V4 模型预热通过。
  - V5/V4 probe 通过。
- `npm run check:i18n`：通过，`520 zh-CN keys match 520 en-US keys`。
- `npm run check:ocr-processing`：通过。
- `npm run build`：通过。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `python -m pytest server\tests`：通过，`27 passed; 1 skipped`。

### 当前风险

- 韩文/阿拉伯文当前会进入完整多候选 fallback，每次约 `5.6–6.3s`，速度还不够“快狠准”。
- 生成式 fixture 已过，但用户真实网页截图、搜索建议、复杂背景和长段落还需要固化为真实 fixture。
- RapidOCR onedir resource 约 `340.7 MB`，可接受但仍需继续裁剪无用依赖和考虑模型分包。
- 覆盖层已原位原字号，但极长译文、复杂背景擦除和画布边界截断仍需下一章继续验收。

### 下一章建议

Chapter 134：把用户真实截图样例固化进 RapidOCR fixture，并优化多语言 fallback 性能。优先尝试 detector 只跑一次、多个 recognizer 复用检测框，减少韩文/阿拉伯文重复 detector 耗时；同时把 selectedLang、candidate quality、总耗时暴露到诊断或 OCR 调试信息。

## Chapter 134：控制台轻量化、录制复用与大模型提示词配置

### 目标

用户在真实构建中反馈三类高优先级问题：

- 每次进入控制台或“识字模型 / 视频录制”都会自动跑启动诊断、RapidOCR 状态和录制依赖检查，页面有卡顿。
- 区域录制保存后提示位置不合理，第二次录制状态没有恢复，打开视频目录无反应，胶囊控制条有明显黑色阴影，重新进入录制时只剩蓝框不能录。
- 大模型翻译配置不能可靠手填自定义模型，界面不应显示 `(New API)`，并需要可编辑、可保存、可传到服务端实际生效的翻译提示词。

### 本章实际处理

- 控制台与配置页轻量化：
  - `useDiagnosticsReport` 默认不再自动调用 `get_diagnostics_report`。
  - `useRapidOcrController` 默认不再自动调用 `get_rapid_ocr_status`。
  - `useRecordingDependencyController` 默认不再自动调用 `get_recording_info`。
  - “识字模型 / 视频录制”的高级录制依赖折叠区默认收起；用户点击刷新、自测或检测可用性时才执行重检查。
- 录制闭环修复：
  - 保存成功后不再立即发送 `recording-ended` 给截图父页面，避免父页面重置导致第二次录制只剩蓝框。
  - 保存成功后录制状态回到 `ready`，保留原蓝色录制框和控制条，可在同一区域继续第二次录制。
  - 开始新录制前清空上一段 `segments`、计时器、保存路径和 notice 窗口。
  - 新增 `recording_notice` 透明提示窗口，保存成功后显示在蓝框中心，短暂展示后自动关闭。
  - “打开视频目录”优先打开已保存视频所在目录；没有保存视频时打开默认 `Videos\YSN`，失败会提示错误。
  - 修复 Tauri ACL：`recording_control` 使用 `openPath(folder)` 需要 `opener:allow-open-path`，否则会报 `plugin:opener|open_path not allowed by ACL`。
  - 将 `recording_notice` 加入默认 capability，保证保存提示窗拥有基础窗口权限。
  - 录制控制条和准备工具条移除黑色阴影。
- 大模型翻译配置修复：
  - 模型字段改为可手填的 `AutoComplete`，获取模型列表只是辅助下拉；模型拉取失败不再阻塞自定义模型。
  - 模型列表拉取成功时只在模型字段为空的情况下自动填第一项，不覆盖用户手填模型。
  - 设置页去掉 `(New API)` 叫法，统一显示“大模型翻译 / LLM Translation”。
  - 新增大模型翻译领域 `newApiDomain` 和提示词 `newApiPrompt`。
  - 前端保存和测试大模型通道时会把 `prompt/domain` 传到翻译服务端。
  - 服务端 `new-api` 配置新增 `prompt/domain` 默认值，旧配置自动 merge。
  - 修复大模型中转地址安全策略：`new-api` 是用户显式配置的自托管/中转服务，允许 LAN、回环或内网解析地址；严格公网校验仍保留给默认公共 provider 路径。
  - `/api/config/fetch_models` 改走用户中转请求路径，`api.yousn.me` 解析到私有/保留 IP 时不再报 `请求地址不合法 (IP 为私有、回环或保留地址)`。
  - 前端“获取模型”失败改为非阻断 warning，并提示不影响手动填写模型。
  - `LLMTranslator` 按 `{{SOURCE_LANGUAGE}}`、`{{TARGET_LANGUAGE}}`、`{{TRANSLATION_DOMAIN}}` 渲染 prompt。
  - LLM 批量翻译改用用户要求的 `%%` 分段协议；如果模型返回段数不匹配，会降级为逐段补译。
  - LLM 缓存 namespace 和服务端 translator cache key 加入 prompt/domain 哈希，避免修改提示词后继续命中旧缓存。
- N100 部署脚本修复：
  - `deploy_n100_translation_server.ps1` 现在同步 `config.py`、`security.py` 和 `translation_prompt.py`，避免远端缺新文件或继续使用旧安全策略。
  - 远端语法检查扩展到 `app.py/config.py/security.py/translator.py/translation_prompt.py`。

### 新增文件

- `server/translation_prompt.py`
- `tauri-client/src/pages/RecordingNoticePage.tsx`
- `tauri-client/src/utils/defaultTranslationPrompt.ts`

### 修改文件

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `server/app.py`
- `server/config.py`
- `server/security.py`
- `server/tests/test_translator.py`
- `server/tests/test_server.py`
- `server/translator.py`
- `deploy_n100_translation_server.ps1`
- `tauri-client/src/components/recording/RecordingControlHud.tsx`
- `tauri-client/src/components/recording/RecordingPrepToolbar.tsx`
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/hooks/useDiagnosticsReport.ts`
- `tauri-client/src/hooks/useRapidOcrController.ts`
- `tauri-client/src/hooks/useRecordingDependencyController.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/main.tsx`
- `tauri-client/src/pages/OcrConfig.tsx`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/pages/Settings.tsx`
- `tauri-client/src-tauri/capabilities/default.json`
- `tauri-client/src/utils/ocrConfigHelpers.ts`
- `tauri-client/src/utils/recordingWindows.ts`

### 删除文件

- 无。

### 本章不做

- 不改 RapidOCR detector/recognizer 质量和速度。
- 不新增新的翻译 provider。
- 不绕过现有文本翻译服务端；本章只是让大模型通道配置真正可控。
- 不承诺录制全场景已完成真实人工验收；本章修复明确代码根因并通过构建检查。

### 验证

- `npm run check:i18n`：通过，`524 zh-CN keys match 524 en-US keys`。
- `npm run build`：通过。
- `python -m pytest server\tests\test_translator.py server\tests\test_server.py`：通过，`16 passed, 2 skipped`。
- `npm run check:ocr-processing`：通过。
- `python -m pytest server\tests`：通过，`26 passed, 3 skipped`。
- `cargo check`：通过。
- ACL 修复后复跑 `npm run build`：通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\deploy_n100_translation_server.ps1 -SkipPublicSmoke`：通过，N100 LAN health 和翻译 smoke 通过。
- N100 `/api/config/fetch_models` 用 `http://127.0.0.1:3001` 验证：不再返回“请求地址不合法”，说明私有中转地址已被允许；真实 `https://api.yousn.me` 当前返回中转服务 `401`，属于 API Key 或中转服务模型列表权限问题。
- `git diff --check`：无空白错误，仅 Windows 换行提示。

### 当前风险

- 录制 notice 窗口位置使用录制选区的逻辑坐标；高 DPI、多屏、负坐标屏幕仍需要真实 Windows 人工验收。
- 保存后继续第二次录制已从状态机上修复，但真实 FFmpeg 二次启动、暂停/继续后再保存、复制视频文件仍需要用户实测确认。
- 大模型 prompt 已能保存并进入服务端请求，但不同模型对 `%%` 协议的遵循程度需要真实 provider 验收。
- 大模型“获取模型”现在允许私有中转地址；若中转服务返回 `401/404`，需要检查 API Key 或该中转是否支持 OpenAI 兼容 `/v1/models`。
- 控制台不再自动跑重诊断后，首次进入页面会偏“未检查”；这是为了避免卡顿，后续可以加轻量缓存态显示。

### 下一章建议

Chapter 135：回到 OCR/翻译真实样例验收。把用户真实截图样例固化进 RapidOCR fixture，重点覆盖清晰英文网页、搜索建议、混排中英、长句和复杂背景；同时做一次录制人工验收清单，确认保存后二次录制、打开目录、复制视频和取消清理都稳定。

## Chapter 135：根目录模型、录制选择器、滚动截图与 DeepL 通道

### 目标

用户要求先完成一组真实使用阻塞项，再继续下一章：

- 清空当前构建缓存和无用文件夹，只保留必要模型和文件。
- 所有 OCR 模型严格放在仓库根目录 `models` 下，旧 `models/ocr` 和 runner 内置模型如果无用就清理。
- 修复录制控制条“打开视频目录”仍被 ACL 拦截的问题。
- 窗口录制和显示器录制进入录制前应先显示可取消的蓝框预览和胶囊目标选择器，而不是直接按鼠标附近窗口开始。
- 首次启动 exe 后自动做一次轻量 readiness 检查，页面进入时只读缓存，避免每次点控制台或识字模型都卡顿。
- 大模型模型列表获取后下拉可正确选择；保存按钮随页面滚动保持可用。
- 主面板窗口发起截图后不应让用户必须从任务栏托盘重新打开面板。
- 新增独立 DeepL 翻译通道。
- 截图工具栏新增“移动”工具，并作为默认模式。
- 滚动截图改为点击选区开始自动滚轮、再次点击停止，并在截取中显示右侧长图预览。

### 本章实际处理

- 根目录 RapidOCR 模型主线：
  - 将 RapidOCR V4/V5 所需模型迁移到仓库根目录 `models/rapidocr`。
  - 当前根目录模型共 `20` 个文件，约 `150 MB`，包含中文检测、方向分类、V4/V5 中文识别、Latin、Korean、Arabic、Cyrillic、Thai 字典和识别模型。
  - 删除旧自研 OCR 路径下的 `models/ocr/**` 工作树文件。
  - 删除 `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/_internal/rapidocr/models/**`，避免 runner 内置模型和根目录模型重复。
  - `.gitattributes` 改为跟踪 `models/rapidocr/**/*.onnx`，不再跟踪旧 `models/ocr/**`。
  - `tauri.conf.json` 资源纳入 `../../models/rapidocr/**/*`，同时保留 runner 和 FFmpeg 资源。
  - `rapidocr_runner.py` 新增 `--model-root`，生产 OCR、probe、warm models、fixture 全部可显式使用根目录模型。
  - Rust RapidOCR 调用统一解析 root `models/rapidocr`，缺模型时返回明确缺失清单，不再静默使用 AppData 或 runner 内嵌目录。
  - RapidOCR 临时图片改写入系统 temp 下的 `ysn-screenshot-translator/rapidocr`，避免 OCR 运行态产物跑到项目模型目录或 AppData 模型目录。
- 启动与状态缓存：
  - Tauri `setup()` 后台运行一次 startup readiness probe。
  - 新增 `get_startup_readiness_snapshot` 和 `run_startup_readiness_probe`。
  - RapidOCR 页面和录制依赖页面优先读取启动缓存；用户手动刷新时才执行重检查。
- 打开视频目录：
  - 新增后端命令 `open_path_in_file_manager(path)`，直接调用系统文件管理器打开路径。
  - 录制控制页、录制依赖页、RapidOCR 模型目录打开都改走该命令，绕开 `plugin:opener|open_path` ACL 限制。
  - Tauri opener capability 同时补充 `$VIDEO/**`、`$APP/**`、`$RESOURCE/**` 作为插件路径兜底。
- 录制目标选择：
  - 新增 `RecordingTargetPicker` 胶囊目标选择器。
  - 点击窗口录制或显示器录制时先进入可预览状态：目标列表横向展示，点击目标会更新蓝框预览，确认后才进入原录制控制条。
  - 目标枚举增加 `exeName`、`processPath`、`iconDataUrl` 字段；本章先用 exe 首字母徽标占位，真实 Shell 图标抽取保留为下一步增强。
  - ESC 会退出录制目标选择和蓝框预览状态。
  - 普通截图工具条在录制目标选择期间隐藏，避免操作冲突。
- 主窗口截图体验：
  - 从主面板发起截图时，截图捕获完成后恢复主窗口可见状态，避免面板消失后只能从托盘重新打开。
- 滚动截图：
  - 滚动模式下点击选区开始采集，程序按选区中心自动模拟鼠标滚轮向下滚动。
  - 截取中再次点击选区停止采集。
  - 截取过程中右侧显示当前拼接预览图。
  - 停止、取消、完成都会清理 timer、帧缓存、预览状态，并恢复窗口 capture exclusion。
- 截图移动工具：
  - 截图工具栏在矩形按钮左侧新增移动工具按钮。
  - 每次进入截图默认是移动模式；选择标注工具后才进入对应绘制/编辑模式。
- DeepL 翻译通道：
  - 服务端新增 `DeepLTranslator`，使用官方 `/v2/translate` 协议和 `DeepL-Auth-Key`。
  - 新增 `deepl` channel 配置：endpoint、api_key、formality。
  - `/api/config/test`、`/api/config/save`、`/api/health`、translator cache key 均支持 DeepL。
  - 前端设置页新增 DeepL 通道卡片，可配置默认 `https://api-free.deepl.com`、API Key 和 formality。
  - `new-api` 模型获取成功后，如果当前字段为空或仍是无效默认值，会自动选择返回列表中的第一个模型；下拉选择会立即写回表单字段。
  - 设置页头部改为 sticky，保存按钮跟随滚动保持可用。
- 测试隔离：
  - DeepL 配置测试执行后恢复原 server config，避免跑测试后用户本机 active channel 被留在 `deepl`。
  - 翻译接口成功测试改为 mock 当前 active translator，不再依赖磁盘配置当前是 Google。
- 清理：
  - 清理忽略构建产物和缓存：`tauri-client/dist`、`tauri-client/src-tauri/target`、`tauri-client/src-tauri/gen`、`server/.pytest_cache`、`__pycache__`、旧根目录 `ocr`、旧根目录 `tauri-client.exe`。
  - 保留 `ffmpeg/ffmpeg.exe`，因为当前录制仍依赖它。

### 新增文件

- `models/rapidocr/arabic_PP-OCRv4_rec_mobile.onnx`
- `models/rapidocr/arabic_PP-OCRv5_rec_mobile.onnx`
- `models/rapidocr/ch_PP-LCNet_x0_25_textline_ori_cls_mobile.onnx`
- `models/rapidocr/ch_PP-OCRv4_det_infer.onnx`
- `models/rapidocr/ch_PP-OCRv4_det_mobile.onnx`
- `models/rapidocr/ch_PP-OCRv4_rec_infer.onnx`
- `models/rapidocr/ch_PP-OCRv4_rec_mobile.onnx`
- `models/rapidocr/ch_PP-OCRv5_det_mobile.onnx`
- `models/rapidocr/ch_PP-OCRv5_rec_mobile.onnx`
- `models/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx`
- `models/rapidocr/ch_ppocr_mobile_v2.0_cls_mobile.onnx`
- `models/rapidocr/cyrillic_PP-OCRv3_rec_mobile.onnx`
- `models/rapidocr/cyrillic_PP-OCRv5_rec_mobile.onnx`
- `models/rapidocr/korean_PP-OCRv4_rec_mobile.onnx`
- `models/rapidocr/korean_PP-OCRv5_rec_mobile.onnx`
- `models/rapidocr/latin_PP-OCRv3_rec_mobile.onnx`
- `models/rapidocr/latin_PP-OCRv5_rec_mobile.onnx`
- `models/rapidocr/ppocr_keys_v1.txt`
- `models/rapidocr/ppocrv5_dict.txt`
- `models/rapidocr/th_PP-OCRv5_rec_mobile.onnx`
- `tauri-client/src/components/recording/RecordingTargetPicker.tsx`

### 修改文件

- `.gitattributes`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `server/app.py`
- `server/config.py`
- `server/tests/test_server.py`
- `server/tests/test_translate_text.py`
- `server/tests/test_translator.py`
- `server/translator.py`
- `tauri-client/scripts/build-rapidocr-runner.ps1`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `tauri-client/src-tauri/capabilities/default.json`
- `tauri-client/src-tauri/rapidocr/rapidocr_runner.py`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src/App.tsx`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/components/screenshot/ScreenshotToolbar.tsx`
- `tauri-client/src/components/settings/SettingsPageHeader.tsx`
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/components/settings/settingsOptions.ts`
- `tauri-client/src/components/settings/types.ts`
- `tauri-client/src/hooks/useRapidOcrController.ts`
- `tauri-client/src/hooks/useRecordingDependencyController.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/pages/Settings.tsx`

### 删除文件

- `models/ocr/**`
- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/_internal/rapidocr/models/**`
- 忽略缓存和构建产物：`tauri-client/dist`、`tauri-client/src-tauri/target`、`tauri-client/src-tauri/gen`、`server/.pytest_cache`、`__pycache__`、旧根目录 `ocr`、旧根目录 `tauri-client.exe`。

### 本章不做

- 不恢复旧自研 `YSN OCR Runtime`。
- 不把真实窗口图标抽取作为本章完成项；当前列表已有字段和 UI 占位，后续可补 Shell/GDI 图标提取。
- 不承诺滚动截图和录制在所有真实 Windows 多屏/DPI 场景已经人工验收；本章完成状态机、命令、构建和自动化验证。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build:rapidocr-runner`：通过。
  - runner 使用 `--model-root C:\Users\ysn\Desktop\zzjt\models\rapidocr` 预热模型。
  - V5 probe 通过，约 `549ms`。
  - V4 probe 通过，约 `863ms`。
  - 重新确认 runner 内部 `rapidocr/models` 不存在，根目录 `models/rapidocr` 为唯一模型根。
- `npm run check:ocr-fixtures`：通过。
  - 中文大字：`2` blocks，约 `1519ms`。
  - 英文 UI：`6` blocks，约 `1537ms`。
  - 小字技术文本：`3` blocks，约 `1465ms`。
  - 韩文：`3` blocks，约 `6247ms`。
  - 日文：`4` blocks，约 `1480ms`。
  - 阿拉伯文：`6` blocks，约 `6591ms`。
  - 日志确认模型均从 `C:\Users\ysn\Desktop\zzjt\models\rapidocr` 读取。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`532 zh-CN keys match 532 en-US keys`。
- `python -m pytest server\tests`：通过，`28 passed, 3 skipped`。
- `npm run build`：通过；仅 Vite chunk 大于 `1200 kB` 的体积警告。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `git diff --check`：无空白错误，仅 Windows 换行提示。
- 测试后确认本机 server config `active_channel` 回到 `google`。

### 当前风险

- 韩文和阿拉伯文仍会进入较重的多候选识别路径，单张 fixture 约 `6s`，后续需要做 detector 复用、候选裁剪和语言先验加速。
- 录制目标列表当前显示 exe 名称和占位徽标，真实窗口图标尚未抽取。
- 滚动截图的窗口排除依赖 Windows 对透明窗口 capture exclusion 的支持；如果系统拒绝排除，可能仍需进一步改成隐藏 overlay 后分帧采集。
- 主窗口截图后恢复可见已从代码层修复，但真实用户习惯上是否需要“截图时主窗口完全不闪”还要人工验收。
- 根目录 `models/rapidocr` 已成为主模型目录；后续发布包和 Git LFS 需要确认 tag/release 流程包含这些文件。

### 下一章建议

Chapter 136：做真实 exe 人工验收和针对性加速。优先验证窗口录制选择器、显示器录制控制条、打开视频目录、二次录制、滚动截图预览/停止/复制、主面板截图恢复；同时开始优化韩文/阿拉伯文 OCR 的 detector 复用和候选数量，目标把复杂脚本路径从 `6s` 降到可接受范围。

## Chapter 136：截图/录制与主设置面板隔离

### 目标

用户提供录制视频 `C:\Users\ysn\Videos\YSN\YSN_20260603_064131.mp4`，反馈：

- 主设置面板存在时，截图启动会闪一下。
- 有时按 `Alt+A` 后还要再点一次鼠标才像真正进入截图状态。
- 主设置面板应该只是一个独立设置面板，不应和正常截图/录制流程产生交叉影响。
- 录制视频中仍能看到设置面板和录制 UI，录制流程观感不稳定。

### 本章实际处理

- 截图启动隔离：
  - `start_screenshot_impl` 不再默认隐藏主窗口、截图后再显示主窗口。
  - Windows 下优先对主窗口使用 `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`，让主设置面板不进入截图捕获，同时避免可见窗口 show/hide 闪烁。
  - 如果系统拒绝 capture exclusion，才降级隐藏主窗口，并在截图完成后恢复。
  - 每次新截图都会隐藏旧 screenshot overlay，不再允许二次热键把旧框选/工具条捕进新截图。
- 快捷键防抖：
  - `start_screenshot` 用 `CAPTURING.swap(true)` 做重入处理。
  - 如果 `Alt+A` 重复触发，会先关闭旧截图/录制边框，再启动新的干净截图会话，避免 overlay 初始化竞态导致“还要补点一下”的半进入状态。
- 录制 UI 隔离：
  - native 录制蓝框新增 `WS_EX_NOACTIVATE`，显示时使用 no-activate，不抢目标窗口焦点。
  - native 录制蓝框创建后也尝试设置 capture exclusion。
  - 录制控制条创建后不再 `setFocus()`，避免从目标窗口抢焦点。
  - 打开录制控制条前，如果主设置面板可见，会临时隐藏主面板，并把 `restoreMainWindow` 写入录制会话 payload。
  - 录制控制条开始录制前强制为 `main`、`screenshot`、`recording_control`、`recording_notice` 设置 capture exclusion。
  - 录制关闭、取消或保存后关闭控制条时恢复 capture exclusion，并按 payload 恢复主设置面板。
- 主面板按钮轻量化：
  - “立即截图”按钮不再显示 loading/success message，避免点击截图时主面板额外重绘。
  - 失败时仍保留错误提示。
- 临时材料：
  - 抽帧检查了用户提供的视频，确认视频中确实录入了主面板/桌面状态；检查完成后删除 `.codex-video-frames` 临时目录。

### 新增文件

- 无。

### 修改文件

- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/App.tsx`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/utils/recordingWindows.ts`

### 删除文件

- 无提交内删除。
- 临时删除未跟踪目录 `.codex-video-frames`。

### 本章不做

- 不切换录制底层到 Windows Graphics Capture；当前仍使用 FFmpeg `gdigrab`。
- 不承诺 `gdigrab` 在所有 Windows 版本都尊重 `WDA_EXCLUDEFROMCAPTURE`；因此录制开始时对主面板采用临时隐藏作为更强兜底。
- 不新增复杂窗口图标抽取。

### 验证

- `npm run build`：通过；仅 Vite chunk 大小警告。
- `cargo check`：通过。
- `npm run check:i18n`：通过，`532 zh-CN keys match 532 en-US keys`。
- `npm run check:ocr-processing`：通过。
- `python -m pytest server\tests`：通过，`30 passed, 1 skipped`。
- `cargo test`：通过，`17 passed; 0 failed`。

### 当前风险

- `SetWindowDisplayAffinity` 对普通 Tauri 主窗口通常可用，但对透明窗口和 FFmpeg `gdigrab` 的表现仍依赖 Windows/DWM/捕获链路；本章通过隐藏主面板降低录制风险。
- 如果用户选择“整屏录制”，录制控制条理论上仍可能被 `gdigrab` 捕获；本章已增加 capture exclusion 和去焦点，但彻底解决需要后续换 Windows Graphics Capture 或设计一个外部控制/全局快捷键停止方案。
- 真实 `Alt+A`、区域录制、窗口录制、显示器录制仍需要用户在当前 exe 中复测。

### 下一章建议

Chapter 137：真实 exe 验收这次窗口隔离。重点测主设置面板打开时 `Alt+A` 是否不闪、不需要补点；录制窗口/显示器时主面板是否不再进入视频；整屏录制控制条是否仍被 `gdigrab` 捕获。如果仍捕获控制条，下一章直接切 Windows Graphics Capture 或增加无控制条热键录制模式。

## Chapter 137：Alt+A 截图不特殊处理主窗口

### 目标

用户进一步明确产品规则：

- `Alt+A` 在主窗口可见时也应该“无视”主窗口。
- 主窗口只是另一个普通软件窗口，负责显示调试和设置，不应被截图系统特殊隐藏、排除或恢复。
- 截图想怎么截都按屏幕真实状态来；主窗口露出来就会被截到，被其他窗口挡住就截不到。

### 本章实际处理

- 调整 `start_screenshot_impl`：
  - 删除对 `main` 主窗口的 `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`。
  - 删除截图前隐藏主窗口、截图后恢复主窗口的 fallback。
  - 保留隐藏旧 screenshot overlay 的逻辑，避免旧截图框/工具栏被下一次截图捕获。
- 产品语义修正：
  - `Alt+A` 不再判断主窗口是否可见、是否前台、是否挡住目标。
  - 主窗口完全按普通屏幕内容处理。

### 新增文件

- 无。

### 修改文件

- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/src-tauri/src/lib.rs`

### 删除文件

- 无。

### 本章不做

- 不改变录制流程中对主窗口的临时隐藏策略；录制是持续捕获，和截图瞬间捕获不是同一类问题。
- 不新增“截图时隐藏主窗口”设置项。
- 不提交、不推送，除非用户再次明确要求。

### 验证

- `cargo check`：通过。
- `npm run build`：通过；仅 Vite chunk 大小警告。

### 当前风险

- 如果主窗口挡住了目标区域，`Alt+A` 会真实截到主窗口，这符合本章规则，但用户需要自己把窗口移开或覆盖。
- “按 Alt+A 后还要补点一下”的焦点问题应因去掉主窗口 hide/exclude 竞态而缓解；真实 exe 仍需要用户复测。

### 下一章建议

Chapter 138：如果复测仍存在“Alt+A 后第一下鼠标不被截图 canvas 接收”，下一章专门处理 screenshot overlay 聚焦和 ready 时序：延迟/取消 `set_focus()`、在前端 ready 后再显示窗口、或用 native overlay 先接管第一下鼠标事件。

## Chapter 138：修复主窗口前台时截图第一下鼠标被吃

### 目标

用户确认 Chapter 137 的主窗口普通窗口规则可用，但仍反馈：

- 点击主窗口后再按 `Alt+A`，截图 overlay 出来后还需要先点一下，第二下才开始框选。

### 本章实际处理

- Rust 前台激活补强：
  - 新增 `activate_webview_window` helper。
  - Windows 下在显示 screenshot overlay 时调用 `BringWindowToTop`、`SetForegroundWindow`、`SetActiveWindow`、`SetFocus`，然后再调用 Tauri `set_focus()`。
  - `overlay_ready_to_show` 显示/激活截图窗口后等待 `35ms`，再重复激活一次，降低 WebView/Windows 前台切换竞态。
- 前端可交互时序修复：
  - 新增 `overlayVisibleRef`。
  - 截图图像、canvas 和 viewport 初始化完成后，先把 `overlayVisibleRef` 和 `overlayVisible` 设为 true，再调用 `overlay_ready_to_show`。
  - `handleMouseDown`、`handleMouseMove`、`handleMouseUp`、`handleDoubleClick` 改用 ref 判断可交互状态，避免第一下鼠标落在 React state 提交前被 `overlayVisible=false` 吃掉。
  - `resetScreenshotState` 同步清理 `overlayVisibleRef`。

### 新增文件

- 无。

### 修改文件

- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/pages/ScreenshotPage.tsx`

### 删除文件

- 无。

### 本章不做

- 不改变 Chapter 137 决策：`Alt+A` 不特殊隐藏或排除主窗口。
- 不改变录制控制条策略。
- 不提交、不推送，除非用户明确要求。

### 验证

- `cargo check`：通过。
- `npm run build`：通过；仅 Vite chunk 大小警告。

### 当前风险

- Windows 仍可能因系统前台窗口策略拒绝某些强制前台调用，但本章同时修了前端 state/ref 竞态，应该明显减少第一下鼠标被吃的问题。
- 还需要用户在真实 exe 中复测主窗口前台、其他窗口前台、托盘唤起等场景。

### 下一章建议

Chapter 139：如果真实 exe 仍吃第一下鼠标，下一步不要继续堆焦点调用，而是改成 screenshot overlay 的 native wrapper 先捕获第一下鼠标，再把坐标派发给 React canvas。

## Chapter 139：RapidOCR 便携 runner 与截图首击再闭环

### 目标

用户换电脑后反馈：

- OCR 报错：`RapidOCR runner failed with status exit code: 1`，stdout 显示 `RapidOCR is not installed. Install rapidocr and onnxruntime, or bundle rapidocr-runner.exe.`
- 主窗口前台时截图仍要先点击一下，第二下才真正开始框选。

### 本章实际处理

- RapidOCR runner 便携性：
  - `resolve_rapidocr_command` 不再只查 exe 同级和 Tauri resource 目录；新增候选表，覆盖 root portable、`resources/rapidocr`、Tauri resource、源码 `src-tauri/resources`、以及根目录 `tauri-client/src-tauri/resources`。
  - runner 查找顺序调整为：显式配置/环境变量、产品内置 runner、最后才回退开发 Python 脚本，避免新电脑没装 Python 包时优先跑裸 `rapidocr_runner.py`。
  - `rapid_ocr_model_root` 改为基于 `AppHandle` 查找模型，覆盖 exe 同级 `models/rapidocr`、Tauri resource `models/rapidocr`、旧 `_up_/_up_/models/rapidocr`、源码根目录模型。
  - `tauri.conf.json` 的 bundle resources 从隐式数组改为显式映射，把 `../../models/rapidocr/` 打包到稳定的 `models/rapidocr/`。
- RapidOCR runner 产物修复：
  - 重新构建 `rapidocr-runner.exe`，修复旧产物缺少 `_socket.pyd`、`select.pyd` 等 Python 标准扩展导致换机启动失败的问题。
  - `build-rapidocr-runner.ps1` 新增隐藏导入和产物校验，构建后必须检查 `_socket*.pyd`、`select*.pyd`，防止再次生成“有 exe 但不能跑”的假 runner。
  - `.gitignore` 放开 `tauri-client/src-tauri/resources/rapidocr/**/*.pyd`，确保 runner 必需 `.pyd` 能纳入仓库/LFS。
  - `check-ocr-fixtures.ps1` 默认优先使用内置 `rapidocr-runner.exe`，不存在时才回退开发 Python 脚本。
- 便携构建脚本：
  - `build.bat` 从 2 步改为 3 步，构建后复制 `tauri-client/src-tauri/resources` 到根目录 `resources`。
  - 构建脚本会检查 runner 和 `models/rapidocr` 是否存在，并提示便携运行必须一并复制 `tauri-client.exe`、`resources`、`models`。
- 截图首击修复：
  - Windows 激活增强：`activate_webview_window` 新增 `GetForegroundWindow`、`GetCurrentThreadId`、`AttachThreadInput`、`ShowWindow(SW_SHOW)`、`SetWindowPos(HWND_TOPMOST)`，再执行 `BringWindowToTop`、`SetForegroundWindow`、`SetActiveWindow`、`SetFocus` 和 Tauri `set_focus()`。
  - 前端 canvas 新增 `tabIndex={-1}`、主动 `focus()`、`onPointerDown/onPointerMove/onPointerUp` 和 pointer capture，减少第一下拖拽被窗口激活过程吞掉的概率。
  - 保持 Chapter 137 产品规则：截图不特殊隐藏或排除主窗口；主窗口仍作为普通屏幕内容处理。

### 新增文件

- RapidOCR runner 新增 Python 3.12 打包运行时文件，包括 `_socket.pyd`、`select.pyd`、`python312.dll`、PIL/numpy/onnxruntime/shapely 等必需扩展文件。

### 修改文件

- `.gitignore`
- `build.bat`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/scripts/build-rapidocr-runner.ps1`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/**`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src/pages/ScreenshotPage.tsx`

### 删除文件

- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/_internal/python311.dll`，由新构建的 `python312.dll` 替代。

### 本章不做

- 不恢复旧自研 `YSN OCR Runtime`。
- 不要求用户安装 Python、RapidOCR 或 onnxruntime 才能使用 OCR。
- 不改变主窗口截图语义；主窗口可见就可被截到，被其他窗口遮挡就不会被截到。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build:rapidocr-runner`：通过。
  - warm models：通过，约 `6821ms`。
  - V5 probe：通过，约 `892ms`。
  - V4 probe：通过，约 `1020ms`。
  - 构建后确认 runner 内部包含 `_socket.pyd` 和 `select.pyd`。
- 直接运行 `src-tauri/resources/rapidocr/rapidocr-runner/rapidocr-runner.exe --probe --model-version v5 --model-root ../../models/rapidocr`：通过，约 `980ms`。
- `npm run check:ocr-fixtures`：通过。
  - 中文大字：`2` blocks，约 `2973ms`。
  - 英文 UI：`6` blocks，约 `2487ms`。
  - 小字技术文本：`3` blocks，约 `2673ms`。
  - 韩文：`3` blocks，约 `8962ms`。
  - 日文：`4` blocks，约 `2418ms`。
  - 阿拉伯文：`6` blocks，约 `9568ms`。
- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过，`532 zh-CN keys match 532 en-US keys`。
- `npm run build`：通过；仅 Vite chunk 大于 `1200 kB` 的体积警告。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。

### 当前风险

- 截图首击问题已做 Windows 激活和 pointer capture 双层修复，但仍需要用户在真实 exe 中验证：主窗口前台、其他窗口前台、托盘唤起、不同显示器/DPI。
- RapidOCR runner 已改为 Python 3.12 打包产物；后续发布前仍应在干净 Windows 机器上做离线 smoke，确认无需 Python 环境。
- 韩文和阿拉伯文 fixture 仍约 `9s`，属于旧风险，后续继续优化复杂脚本候选数量和 detector 复用。
- `build.bat` 现在会生成根目录 `resources`；便携迁移时必须和 `models` 一起复制，单独复制 exe 仍不是完整发布包。

### 下一章建议

Chapter 140：做干净机器/干净目录真实 smoke。用 `build.bat` 生成根目录 exe 和 `resources` 后，把 `tauri-client.exe`、`resources`、`models` 复制到一个不含源码的新目录，验证 OCR 自测、真实截图 OCR、主窗口前台 `Alt+A` 首击框选、多屏/DPI 场景。如果首击仍失败，下一章实现 native first-click relay：由原生窗口先捕获首个鼠标按下坐标并派发给 React canvas。

## Chapter 140：便携打包脚本与构建缓存清理

### 目标

用户反馈：

- `build.bat` 当前构建不了，CMD 输出出现 `/3] 复制产物 ...`、`[错误]` 被当成命令等批处理解析错误。
- 希望调整打包逻辑。
- 希望清空当前文件夹中无用的构建缓存、临时文件和旧产物。

### 本章实际处理

- `build.bat` 重写为稳定便携构建脚本：
  - 输出目录改为 `release\YSN-Screenshot-Translator`，不再把 exe、resources 散落到项目根目录。
  - 不再因为根目录旧 `tauri-client.exe` 被占用而阻塞构建；旧根目录 exe 删除失败只提示，新产物仍输出到 `release`。
  - 构建前检查 `package.json`、`tauri.conf.json`、内置 RapidOCR runner 和 `models/rapidocr`。
  - 构建后复制 `tauri-client.exe`、`resources`、`models/rapidocr` 到同一个便携目录。
  - 支持 `--no-pause`，方便自动化验证。
  - 使用无 BOM UTF-8 + CRLF 重写，修复 CMD 因 LF 行尾导致的批处理碎片解析问题。
- `pack_release.ps1` 重写为安全 zip 脚本：
  - 默认打包 `release\YSN-Screenshot-Translator`。
  - 可选 `-Build` 先调用 `build.bat --no-pause`。
  - 生成 `release\ScreenshotTranslator_Windows.zip`，若传 `-Version` 则带版本后缀。
  - 不再 commit、tag、push 或发布 GitHub Release，避免打包脚本误动 Git 历史。
- `.gitignore` 更新：
  - 忽略 `/release/` 和 `/ScreenshotTranslator_Windows*.zip`，防止把便携成品和 200MB zip 误提交。
- 当前文件夹清理：
  - 删除 `tauri-client\dist`、`tauri-client\src-tauri\target`、`tauri-client\src-tauri\gen`。
  - 删除 `server\.pytest_cache` 和项目内 `__pycache__`。
  - 删除旧 `release` 后重新生成干净成品。
  - 保留 `models\rapidocr`、`ffmpeg`、`tauri-client\src-tauri\resources\rapidocr`、`tauri-client\node_modules`。

### 新增文件

- 无代码新增文件。
- 生成但被忽略的成品：
  - `release\YSN-Screenshot-Translator\**`
  - `release\ScreenshotTranslator_Windows.zip`

### 修改文件

- `.gitignore`
- `build.bat`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `pack_release.ps1`

### 删除文件

- 构建缓存/临时目录：
  - `tauri-client\dist`
  - `tauri-client\src-tauri\target`
  - `tauri-client\src-tauri\gen`
  - `server\.pytest_cache`
  - 项目内 `__pycache__`
- 旧未跟踪输出：
  - 旧 `release` 目录先删除后重新生成。

### 本章不做

- 不删除 `models\rapidocr`、内置 RapidOCR runner、`ffmpeg` 或 `node_modules`。
- 不提交、不推送、不打 tag。
- 不把 zip 发布到 GitHub Release。

### 验证

- `cmd /c "build.bat --no-pause"`：通过。
  - 成功生成 `release\YSN-Screenshot-Translator\tauri-client.exe`。
  - 复制 `resources` 与 `models\rapidocr` 到便携目录。
  - Vite 仍只有 chunk 大于 `1200 kB` 的体积警告。
- 便携目录 runner 自测：通过。
  - 命令：`release\YSN-Screenshot-Translator\resources\rapidocr\rapidocr-runner\rapidocr-runner.exe --probe --model-version v5 --model-root release\YSN-Screenshot-Translator\models\rapidocr`
  - 结果：`status=success`，约 `789ms`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过。
  - 生成 `release\ScreenshotTranslator_Windows.zip`。
  - zip 大小约 `210.93 MB`。
- 打包后再次删除构建缓存：
  - `tauri-client\dist`：不存在。
  - `tauri-client\src-tauri\target`：不存在。
  - `tauri-client\src-tauri\gen`：不存在。
  - `server\.pytest_cache`：不存在。

### 当前风险

- `release` 是被忽略的本地成品目录；如果清理整个 ignored 输出，需要先备份 zip 或重新运行 `build.bat`。
- 因为打包后清理了 `target` 和 `dist`，下一次构建会重新编译 Rust，耗时会更长，但当前项目目录更干净。
- 仍需要用户在真实 UI 中复测主窗口前台 `Alt+A` 是否首击即可框选。

### 下一章建议

Chapter 141：做真实便携目录 smoke。直接运行 `release\YSN-Screenshot-Translator\tauri-client.exe`，验证配置中心 OCR 自测、真实截图 OCR、主窗口前台 `Alt+A` 首击框选、复制/保存、以及把整个 `release\YSN-Screenshot-Translator` 目录复制到干净位置后仍可离线运行。

## Chapter 141：RapidOCR 常驻 worker、小字增强与真实首击 smoke

### 目标

用户决定采用 D 方案，并授权无人连续执行到可用为止。本章目标是一次性闭合：

- RapidOCR 从一次性进程升级为可控的长期 JSON-RPC worker，降低重复 OCR 冷启动延迟。
- 小字技术文本识别增强，减少 `PixPinDaemon.exe`、`localsend-cli.exe` 这类短小标识被漏识别或拆坏。
- 配置面板可以控制常驻 OCR 加速开关，并显示 worker 状态。
- 修复主窗口前台时 `Alt+A` 后仍要先点击一下才能框选的问题。
- 刷新便携打包产物、生成 zip，并清理当前目录构建缓存。

### 本章实际处理

- Python RapidOCR runner：
  - `rapidocr_runner.py` 保留原 CLI / probe / one-shot OCR，同时新增 `--worker` JSONL 模式。
  - worker 支持 `ping`、`status`、`warm`、`ocr`、`shutdown`。
  - worker 内部按 `(lang, version, modelRoot)` 缓存 RapidOCR engine；预热后重复识别不再重新初始化模型。
  - stdout 只输出 JSONL 响应，RapidOCR 日志重定向到 stderr，避免污染协议。
  - stdin/stdout/stderr 固定 UTF-8，并兼容 PowerShell pipeline 可能带入的 UTF-8 BOM。
  - OCR 候选增加小字增强图：padding、2x/3x Lanczos upscale、autocontrast、contrast、sharpen，并把增强图坐标缩回原图。
  - 自动路由优先中文/英文快速候选，低质量时才进入小字增强和复杂脚本 fallback。
- Rust 后端：
  - 新增 RapidOCR worker 进程管理、请求/响应、启动、停止、重启、状态查询和预热逻辑。
  - `run_rapidocr_sync` 默认优先走 worker，worker 失败自动回退 one-shot runner。
  - `prewarm_local_ocr_models` 在配置允许时启动并预热 worker。
  - `get_rapid_ocr_status` 不再因为普通状态刷新就冷启动 OCR probe；worker 状态单独暴露。
  - 应用退出清理时停止 worker，避免残留子进程。
  - 新增 `get_screenshot_pointer_state`，通过 Win32 获取当前鼠标左键和截图窗口相对坐标，为首击恢复提供 native relay。
- 前端配置中心：
  - 新增 `rapidOcrWorkerEnabled` 配置，默认开启。
  - RapidOCR 面板新增“常驻 OCR 加速”开关，以及启动/停止/重启按钮。
  - 面板显示 worker 是否启用、是否运行、PID、已缓存模型和最近错误。
  - 修复 RapidOCR 面板和 readiness overview 中的乱码文案。
- 截图首击修复：
  - `ScreenshotPage` 在 overlay ready 后短时间轮询 native pointer state；如果用户已经按住左键，则直接从 native 坐标开始框选。
  - `mousemove` 也加入兜底：当检测到左键按住但 React 未收到 `mousedown` 时，立即创建普通框选。
  - 复用 `startPlainSelectionAt`，避免把首击恢复逻辑复制到多个事件分支。
- OCR 文本后处理：
  - `restoreCollapsedUiTextSpacing` 增加技术文件名保护，避免把 `PixPinDaemon.exe` 拆成 `Pix Pin Daemon.exe`。
  - 合并 OCR 常见拆词：`localsend-cli. exe` 会恢复为 `localsend-cli.exe`。
  - 更新 `check-ocr-processing` 断言，确保 `.exe` 技术标识后续不回归。
- 构建与清理：
  - 重新跑 `build.bat --no-pause` 刷新 `release\YSN-Screenshot-Translator`。
  - 重新生成 `release\ScreenshotTranslator_Windows.zip`。
  - 清理 `tauri-client\dist`、`tauri-client\src-tauri\target`、`tauri-client\src-tauri\gen`、`server\.pytest_cache`、项目内 `__pycache__`、烟测临时图片和临时脚本。
  - 保留 `release`、`models\rapidocr`、内置 RapidOCR runner、`ffmpeg` 和 `node_modules`。

### 新增文件

- RapidOCR runner onedir 产物中新增/更新 Python 3.12 运行时依赖文件，包括 `python312.dll`、PIL、numpy、onnxruntime、OpenCV、shapely、yaml、bidi 和多个 `.pyd` 扩展。

### 修改文件

- `.gitignore`
- `build.bat`
- `pack_release.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/scripts/build-rapidocr-runner.ps1`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/src-tauri/rapidocr/rapidocr_runner.py`
- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/**`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src/components/config/ConfigReadinessOverview.tsx`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/hooks/useOcrConfigController.tsx`
- `tauri-client/src/hooks/useRapidOcrController.ts`
- `tauri-client/src/ocr-models/types.ts`
- `tauri-client/src/ocr-processing/textSpacing.ts`
- `tauri-client/src/pages/OcrConfig.tsx`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/utils/ocrConfigHelpers.ts`

### 删除文件

- 构建缓存/临时目录：
  - `tauri-client\dist`
  - `tauri-client\src-tauri\target`
  - `tauri-client\src-tauri\gen`
  - `server\.pytest_cache`
  - 项目内 `__pycache__`
- 临时 smoke 文件：
  - `C:\ysn-ocr-smoke` junction
  - `%TEMP%\ysn-worker-smoke`
  - `%TEMP%\ysn-first-click-smoke.png`
  - `%TEMP%\ysn-*-smoke.ps1`
- RapidOCR runner 旧 Python 3.11 动态库已由 Python 3.12 runner 替换。

### 本章不做

- 不恢复旧自研 `YSN OCR Runtime`。
- 不把 PaddleOCR-json 重新作为普通主路径。
- 不实现用户明确说暂不需要的 watchdog 自动重启策略。
- 不提交、不推送、不打 tag。
- 不发布 GitHub Release。

### 验证

- `python -m py_compile tauri-client/src-tauri/rapidocr/rapidocr_runner.py`：通过。
- `npm run build:rapidocr-runner`：通过，已重建内置 `rapidocr-runner.exe`。
- release runner probe：通过。
  - 命令：`release\YSN-Screenshot-Translator\resources\rapidocr\rapidocr-runner\rapidocr-runner.exe --probe --model-version v5 --model-root release\YSN-Screenshot-Translator\models\rapidocr`
  - 结果：`status=success`，约 `1349ms`。
- release worker JSONL：通过。
  - PowerShell ASCII ping/shutdown：`ping` 和 `shutdown` 均返回 `ok=true`。
  - Python UTF-8 stdin + 中文路径：`warm` 预热 `ch`/`latin` 成功，约 `1451ms`。
  - 预热后小字技术样例：第一次 OCR 约 `1692ms`，第二次约 `1026ms`，`selected_init_ms=0`，识别到 `PATH=C:\Windows\System32`、`PixPinDaemon.exe`、`localsend-cli.exe`。
- `npm run check:ocr-fixtures`：通过。
  - 中文大字、英文 UI、小字技术文本、韩文、日文、阿拉伯文均通过。
  - 本次 one-shot fixture 中韩文约 `17849ms`、阿拉伯文约 `14699ms`，复杂脚本仍是后续性能重点。
- `npm run check:ocr-processing`：通过，新增 `.exe` 技术标识合并断言。
- `npm run check:i18n`：通过，`532 zh-CN keys match 532 en-US keys`。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `cmd /c "build.bat --no-pause"`：通过。
  - 成功生成 `release\YSN-Screenshot-Translator\tauri-client.exe`。
  - 成功复制 `resources` 和 `models\rapidocr` 到便携目录。
  - Vite 仍只有 chunk 大于 `1200 kB` 的体积警告。
- 真实 release UI smoke：通过。
  - 启动 `release\YSN-Screenshot-Translator\tauri-client.exe` 成功，进程约 `27.3 MB` working set 初始占用。
  - 主窗口前台时，自动发送 `Alt+A` 后约 `120ms` 立即按住拖拽，截图 overlay 成功生成 `220 x 120` 选区和工具条。
  - 在同一真实选区按 `Ctrl+D`，弹出 `OCR Result` 窗口，剪贴板获得 OCR 文本：`图 / 咖手 / 入颜色，选择标准颜色可增加`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`：通过。
  - i18n、OCR processing、前端 build、Rust check、Rust tests 均通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过。
  - 生成 `release\ScreenshotTranslator_Windows.zip`。
  - zip 大小约 `211.98 MB`。
- 清理后确认：
  - `tauri-client\dist`：不存在。
  - `tauri-client\src-tauri\target`：不存在。
  - `tauri-client\src-tauri\gen`：不存在。
  - `server\.pytest_cache`：不存在。

### 当前风险

- 常驻 worker 能显著降低重复识别冷启动，但仍会常驻一份 Python / ONNXRuntime / RapidOCR 内存；用户可在 RapidOCR 面板关闭“常驻 OCR 加速”以换取更低常驻占用。
- 韩文、阿拉伯文等复杂脚本 one-shot fixture 仍慢，主要因为 fallback 会初始化并尝试多个识别模型；下一章应优先做候选早停、检测框复用或更明确的脚本路由。
- 自动化已复现并通过主窗口前台首击拖拽，但不同 DPI、多屏、杀毒软件、远程桌面环境仍建议用户回来后手工复测。
- `release` 和 zip 是本地忽略成品；如果后续清理 ignored 文件，需要先备份或重新运行打包脚本。

### 下一章建议

Chapter 142：继续做复杂脚本和真实截图样例闭环。优先把用户“小字翻译不了”的真实截图场景固化为 fixture，再优化韩文/阿拉伯文 fallback 的候选早停和检测框复用；同时补一个发布前人工验收清单，覆盖多屏/DPI、真实网页、OCR 结果窗、翻译覆盖层、复制/保存和录制主流程。

## Chapter 142 - 截图启动热路径与翻译等待优化（2026-06-03）

### 目标

- 继续检查用户反馈的“翻译还要等很久”和 `Alt+A` 明显延迟。
- 在不改变 RapidOCR 主线和用户配置的前提下，先优化截图启动热路径、选区裁剪和翻译请求等待策略。
- 重新验证最终便携包，确保这次优化进入 `release\YSN-Screenshot-Translator` 和 zip。

### 实际完成

- 截图启动热路径：
  - `start_screenshot_impl` 不再常规通过 Tauri event 传输整张全屏 PNG 的 base64 字符串。
  - 后端捕获全屏后把 PNG 写入本地应用数据目录，并向前端发送 `{ kind: "file", path, bytes }` payload。
  - 前端通过 Tauri asset protocol 加载本地截图文件；文件写入失败时仍保留旧 base64 fallback。
  - 启用 `tauri.conf.json` 的 `assetProtocol`，scope 限制在 `$LOCALDATA/ScreenshotTranslator/**`。
- Overlay 首屏显示：
  - 前端兼容 file/base64 两种 `screenshot-updated` payload。
  - 移除截图图片加载后等待多帧稳定 viewport 的阻塞等待。
  - `overlay_ready_to_show` 改为非阻塞通知，避免额外等待后端短 sleep。
- 翻译裁剪热路径：
  - `captureRegionBase64` 优先用前端已加载的全屏截图 `imageRef` 在 canvas 内直接裁剪当前选区。
  - 如果浏览器安全限制或 canvas 裁剪失败，再 fallback 到 Rust `capture_region`。
  - 这样翻译/OCR 不再每次都要求 Rust 解码整张全屏 PNG 后再裁剪。
- 翻译服务等待：
  - 多候选服务 URL 时改为 700ms 延迟对冲，请求先到先用，避免内网地址慢失败时长时间挡住公网回落。
  - 默认文本翻译超时从 20 秒收紧到 9 秒。
  - 未翻译 Latin 行的二次补救请求最多等待 5 秒，避免极端慢网把用户卡住太久。
  - 翻译链路新增 console timing，包括选区裁剪、OCR、翻译、重试、渲染、服务端 timings、命中 server/channel 和 block 数。
- 当前配置诊断：
  - 本机当前只配置公网 `https://ocr.yousn.me`，未启用 `lanServerUrl`，因此多 URL 对冲代码不会在当前配置下触发。
  - 公网 `/api/health` 实测约 `1035ms`。
  - 公网 `/api/translate_text` 实测冷请求约 `1865ms`，服务端 provider 约 `389ms`；服务端缓存命中后客户端总耗时仍约 `513–610ms`，这是当前网络往返/连接成本的主要地板。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src-tauri/Cargo.lock`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无代码文件删除。
- 构建缓存/临时目录：
  - `tauri-client\dist`
  - `tauri-client\src-tauri\target`
  - `tauri-client\src-tauri\gen`
- 临时 smoke/timing 脚本：
  - `%TEMP%\ysn-alt-a-debug.ps1`
  - `%TEMP%\ysn-alt-a-timing.ps1`
  - `%TEMP%\ysn-first-click-after-file-simple.ps1`
  - `%TEMP%\ysn-first-click-after-file.ps1`
  - `%TEMP%\ysn-final-alt-a-timing.ps1`

### 本章不做

- 不覆盖用户本机 `config.json`，不擅自写入 LAN 服务地址或 token。
- 不恢复旧自研 `YSN OCR Runtime`。
- 不恢复 PaddleOCR-json 作为普通主路径。
- 不做 watchdog 自动重启。
- 不提交、不推送、不打 tag。

### 验证

- `npm run check:ocr-processing`：通过。
- `npm run check:i18n`：通过。
- `npm run build`：通过，Vite 仍只有 chunk 大于 `1200 kB` 的体积警告。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `npm run check:ocr-fixtures`：通过。
  - 中文、英文 UI、小字技术文本、韩文、日文、阿拉伯文 fixture 均通过。
  - 本轮韩文约 `13964ms`、阿拉伯文约 `14507ms`，复杂脚本仍是后续性能重点。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`：通过。
  - i18n、OCR processing、前端 build、Rust check、Rust tests 均通过。
- `cmd /c "build.bat --no-pause"`：通过。
  - 成功生成 `release\YSN-Screenshot-Translator\tauri-client.exe`。
  - 成功复制 `resources` 和 `models\rapidocr` 到便携目录。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过。
  - 生成 `release\ScreenshotTranslator_Windows.zip`，大小约 `211.36 MB`。
- 真实 release `Alt+A` 秒表：通过。
  - 启动最终发布 exe 后连续 3 次发送 `Alt+A`。
  - 截图辅助窗口可见耗时分别约 `271ms`、`223ms`、`244ms`。
- 当前公网翻译服务 timing：通过。
  - health 约 `1035ms`。
  - 冷请求客户端总耗时约 `1865ms`，服务端 `provider_ms=389`。
  - 缓存命中客户端总耗时约 `513–610ms`，服务端 `total_ms=0`。
- 清理后确认：
  - `tauri-client\dist`：不存在。
  - `tauri-client\src-tauri\target`：不存在。
  - `tauri-client\src-tauri\gen`：不存在。
  - `server\.pytest_cache`：不存在。

### 当前风险

- `Alt+A` 已从明显等待压到约 `0.22–0.27s`，但仍包含屏幕捕获和 PNG 编码成本；要继续低于 150ms 需要更深的 raw buffer / 更快图像传输方案，改动风险更高。
- 当前翻译慢的主要瓶颈是公网 RTT/握手和远端服务链路；代码已支持 LAN 优先和对冲，但用户当前配置只有公网，所以实际仍会受公网网络波动影响。
- 韩文、阿拉伯文等复杂脚本 fixture 仍慢，原因仍是多模型 fallback 初始化和候选尝试，下一章需要做脚本路由早停、检测框复用或更细的候选评分。
- Tauri asset protocol 现在只允许加载 `$LOCALDATA/ScreenshotTranslator/**`，后续如果截图缓存目录变化，需要同步更新 scope。

### 下一章建议

Chapter 143：继续做翻译“等待感”体验优化。优先在设置页/状态栏暴露当前实际使用的翻译 URL、LAN 优先状态和最近耗时；为真实翻译覆盖层做自动 smoke；如果用户愿意配置 LAN 服务，启用 `lanServerUrl` 后复测对冲效果。同时继续推进复杂脚本 fallback 的早停和 detector 复用。

## Chapter 143 - 论坛列表 OCR 清洗与翻译质量守门（2026-06-03）

### 目标

- 按用户确认的“轻、快、准、段落整齐”路线，先修复真实论坛列表截图里的明显 OCR 噪声和坏译入口。
- 把根目录 `测试图片\1.png` 到 `测试图片\4.png` 纳入可复跑 OCR fixture，避免只凭截图目测。
- 在不引入默认 VLM、不恢复旧 OCR 主路径的前提下，补上低风险的版面 profile、技术实体保护和坏译裁判。

### 实际完成

- 新增论坛/技术列表 OCR profile：
  - 过滤 `■`、`□`、`×`、箭头、星标、序号圆点等纯图标 OCR block。
  - 清理 `1 Codex`、`■Codex`、`■ API`、`1 Feedback` 这类由图标污染出来的前缀。
  - 修复 `ChatGPTApps SDK`、`ChatGPTApps SDKmcp`、`OpenAl`、`APls`、`Al-generated`、`Cant` 等高频 OCR 误读。
  - 合并短换行标题续行，例如 `Codex Desktop and Codex` + `Mobile`、`May` + `9th`，避免翻译时被拆成孤立词。
- 强化翻译质量保护：
  - 翻译 prompt 的 protected terms 增加 `Codex`、`OpenAI`、`ChatGPT`、`API`、`APIs`、`SDK`、`MCP`、`GPT-5`、`VLM`。
  - 客户端翻译结果 normalize 增加轻量坏译修复：`Codex` 不允许变成“法典/科德克斯”，论坛语境 `ticket` 修为“工单”，测试语境 `fixture` 修为“固定测试样例”，`VLM fallback` 修为“VLM 兜底识别”。
  - 翻译质量摘要新增 `badTranslationCount`、`badTranslationIndexes`、`badTranslationReasons`，可标记译文丢失受保护实体的情况。
- 扩展 OCR fixture：
  - `check-ocr-fixtures.ps1` 默认检测根目录 `测试图片` 是否存在；存在时自动追加 `1.png` 到 `4.png` 的真实截图门禁，不存在则不阻断其他环境。
  - 使用 Unicode code point 生成默认中文目录名，避免 Windows PowerShell 5.1 读取 `.ps1` 时中文路径乱码。
  - 四张用户图分别断言 OpenAI 社区页面、详情页、标签小图和论坛列表图的核心文本。

### 新增文件

- `tauri-client/src/ocr-processing/forumListProfile.ts`

### 修改文件

- `tauri-client/src/ocr-processing/blockFilters.ts`
- `tauri-client/src/ocr-processing/index.ts`
- `tauri-client/src/ocr-processing/normalizationReport.ts`
- `tauri-client/src/ocr-processing/translationPolicy.ts`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无。

### 本章不做

- 不默认启用 VLM。
- 不接入浏览器 DOM/UIA 文本源抢跑。
- 不改用户翻译服务地址、token 或本机配置。
- 不重建发布包、不提交、不推送、不打 tag。

### 验证

- `npm run check:ocr-processing`：通过。
  - 覆盖论坛图标前缀清洗、`ChatGPT Apps SDK mcp` 修复、标题续行合并、受保护技术实体、坏译修复和坏译质量标记。
- `npm run check:ocr-fixtures`：通过。
  - 生成式中文、英文 UI、小字技术文本、韩文、日文、阿拉伯文 fixture 均通过。
  - 新增本地用户图 fixture 均通过：
    - `user-1`：92 blocks，约 `5181ms`。
    - `user-2`：21 blocks，约 `3571ms`。
    - `user-3`：8 blocks，约 `2135ms`。
    - `user-4`：18 blocks，约 `2599ms`。
- `npm run build`：通过，Vite 仍只有 chunk 大于 `1200 kB` 的体积警告。
- `npm run check:i18n`：通过，`532 zh-CN keys match 532 en-US keys`。

### 当前风险

- 论坛列表 profile 是低风险规则层，只处理当前已见到的图标污染、常见 OCR 误读和短续行合并；更复杂网页/文档仍需要后续 layout composer。
- 坏译修复不是千万词典路线，只是守住高价值技术实体和已知灾难译法；长尾术语仍应靠动态术语注入、上下文翻译和质量裁判继续扩展。
- `check-ocr-fixtures.ps1` 会在本机存在 `测试图片` 时额外跑四张真实图，耗时增加但能换取真实样例回归；无该目录的环境不会阻断。
- 真实翻译覆盖层的视觉效果仍需下一章做自动/人工 smoke，尤其是合并后的多行标题区域。

### 下一章建议

Chapter 144：把当前实际翻译服务 URL、LAN 优先状态、最近耗时和本轮 OCR normalization 摘要暴露到 UI/诊断；同时为 `测试图片\4.png` 做一条真实翻译覆盖层 smoke，确认清洗后的 block 进入翻译与渲染，而不是只在文本门禁中通过。

## Chapter 144 - UIA 文本源抢跑、翻译预热诊断与构建脚本闭环（2026-06-03）

### 目标

- 在论坛列表清洗基础上继续降低翻译等待感：真实页面能走文本源时不要再等 OCR。
- 把翻译服务当前 URL、耗时、缓存和模型信息暴露出来，避免用户只看到“在线 700ms+”但不知道慢在哪里。
- 修复 `build.bat` 在 Windows cmd 下的解析失败，并重新验证便携 release。

### 实际完成

- 非阻塞 Windows UI Automation 文本源抢跑：
  - `Alt+A` 进入截图时记录前台窗口元信息，并在后台用 UIA 收集可见文本元素。
  - 前端翻译动作等待文本源最多 `80ms`；命中足够文本则直接翻译文本源 block，跳过 RapidOCR；未命中自动回落 RapidOCR。
  - 该路径只做加速，不阻塞 OCR 主线。
- 翻译服务预热与诊断：
  - 截图页加载、进入翻译模式和执行翻译时预热 `/api/health`。
  - 翻译后浮层显示来源、OCR 耗时、翻译耗时、服务端耗时、provider/model、缓存命中和实际服务 URL。
  - 预热结果只缓存短时间，避免 UI 显示陈旧服务状态。
- 截图上下文感知翻译 prompt：
  - 翻译请求携带本轮 OCR/text-source 全局上下文。
  - 对软件/支持场景注入 `ticket`、`fixture`、`fallback`、`issue`、`bug` 等动态术语提示。
- `build.bat` 修复：
  - 批处理改为 ASCII 文案并保持 CRLF，避免中文编码和 LF 导致 cmd 标签/括号块错乱。
  - 构建产物统一输出到 `release\YSN-Screenshot-Translator`，再次强调迁移时复制整个便携目录。

### 新增文件

- `tauri-client/src-tauri/src/text_source.rs`

### 修改文件

- `build.bat`
- `pack_release.ps1`
- `tauri-client/src-tauri/Cargo.lock`
- `tauri-client/src-tauri/Cargo.toml`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `tauri-client/src/ocr-processing/translationPolicy.ts`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无源码文件删除。
- 构建后清理了生成缓存：
  - `tauri-client\dist`
  - `tauri-client\src-tauri\target`
  - `tauri-client\.tmp-ocr-processing-check`

### 本章不做

- 不实现 OCR worker watchdog。
- 不恢复旧自研 `YSN OCR Runtime`。
- 不恢复 PaddleOCR-json 作为普通主路径。
- 不修改用户本机翻译服务 token、密钥或私有配置。
- 不提交、不推送、不打 tag。

### 验证

- `npm run check:ocr-processing`：通过。
- `npm run build`：通过，Vite 仍只有 chunk 大于 `1200 kB` 的既有体积警告。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `npm run check:ocr-fixtures`：通过，包含根目录 `测试图片\1.png` 到 `测试图片\4.png`。
- `npm run smoke:translate-service`：通过，公网 `https://ocr.yousn.me` 总计 15 blocks / 5 batches 约 `3623ms`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`：通过。
- `cmd /c "build.bat --no-pause"`：通过，生成 `release\YSN-Screenshot-Translator\tauri-client.exe`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过，生成 `release\ScreenshotTranslator_Windows.zip`，约 `211.05 MB`。
- release smoke：`release\YSN-Screenshot-Translator\tauri-client.exe` 可启动。
- release `Alt+A` smoke：自动发送 `Alt+A` 后截图辅助窗口可见约 `106ms`。

### 当前风险

- UIA 文本源对浏览器、Electron、原生 Win32、远程桌面和不同权限窗口的命中率不同，仍必须保留 RapidOCR 回落。
- 当前公网翻译仍可能被 TLS/TTFB/服务端链路拉到 `700ms+`，即使模型推理本身只有数百毫秒。
- OCR 长段英文已经能被行合并为 7 行，但仍缺段落级合并，长段翻译会被拆得不自然。

### 下一章建议

Chapter 145：修复用户 `测试图片\测试2\原始文本.png` 的长段英文翻译效果，加入正文段落 profile；同时把根目录的“程序图标/任务栏图标”接入 Tauri 图标资源，刷新图标缓存并重新打包。

## Chapter 145 - 双图标接入、正文段落 OCR 合并与测试图片闭环（2026-06-03）

### 目标

- 按用户给出的根目录 `程序图标.ico` 和 `任务栏图标.ico` 替换应用图标与托盘/任务栏小图标，并刷新 Windows 图标缓存。
- 解决 `测试图片\测试2\原始文本.png` 这类长段英文截图被拆成碎词/碎行，导致译文比微信差很多的问题。
- 详细跑根目录 `测试图片` 和 `测试图片\测试2`，把样例纳入可复跑门禁。

### 实际完成

- 双图标资源：
  - 用根目录 `程序图标.ico` 生成 Tauri 应用图标资源：`32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.png`、`icon.ico` 和 Windows `Square*Logo.png` / `StoreLogo.png`。
  - 用根目录 `任务栏图标.ico` 生成专用托盘图标：`taskbar-16x16.png`、`taskbar-32x32.png`、`taskbar-64x64.png`、`taskbar.png`、`taskbar.ico`。
  - Rust 托盘图标从通用 `32x32.png` 改为专用 `taskbar-32x32.png`。
  - 将新 `程序图标.ico` 同步为根目录 `app.ico`，保留旧文档/脚本兼容入口。
- 正文段落 OCR 合并：
  - 新增 `paragraphProfile`，在多行、同左边界、行距紧密、总文本像正文时，把 OCR 虚拟行合成一个段落 block。
  - 英文正文按空格合并并恢复技术/文件名间距；中日韩正文按连续文本合并，避免中文段落中间插入空格。
  - 论坛/列表图仍由 `forumListProfile` 处理；段落规则只在连续正文条件满足时触发，避免把帖子列表误合并。
- 测试图片闭环：
  - `check-ocr-fixtures.ps1` 默认继续检测 `测试图片\1.png` 到 `测试图片\4.png`。
  - 新增 `测试图片\测试2` 三张图的真实 OCR fixture：`原始文本.png`、`微信翻译结果.png`、`我们的截图翻译结果.png`。
  - 实测 `原始文本.png` 原始 OCR 为 `45` 个词级 block；前端 normalization 后变为 `1` 个正文段落。
  - 实测 `测试图片\4.png` 原始 OCR 为 `18` 个 block；normalization 后为 `16` 个论坛列表 block，没有被正文段落规则误合并。

### 新增文件

- `tauri-client/src/ocr-processing/paragraphProfile.ts`
- `tauri-client/src-tauri/icons/taskbar-16x16.png`
- `tauri-client/src-tauri/icons/taskbar-32x32.png`
- `tauri-client/src-tauri/icons/taskbar-64x64.png`
- `tauri-client/src-tauri/icons/taskbar.png`
- `tauri-client/src-tauri/icons/taskbar.ico`

### 修改文件

- `app.ico`
- `tauri-client/src-tauri/icons/32x32.png`
- `tauri-client/src-tauri/icons/128x128.png`
- `tauri-client/src-tauri/icons/128x128@2x.png`
- `tauri-client/src-tauri/icons/icon.ico`
- `tauri-client/src-tauri/icons/icon.png`
- `tauri-client/src-tauri/icons/Square30x30Logo.png`
- `tauri-client/src-tauri/icons/Square44x44Logo.png`
- `tauri-client/src-tauri/icons/Square71x71Logo.png`
- `tauri-client/src-tauri/icons/Square89x89Logo.png`
- `tauri-client/src-tauri/icons/Square107x107Logo.png`
- `tauri-client/src-tauri/icons/Square142x142Logo.png`
- `tauri-client/src-tauri/icons/Square150x150Logo.png`
- `tauri-client/src-tauri/icons/Square284x284Logo.png`
- `tauri-client/src-tauri/icons/Square310x310Logo.png`
- `tauri-client/src-tauri/icons/StoreLogo.png`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/ocr-processing/index.ts`
- `tauri-client/src/ocr-processing/normalizationReport.ts`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无源码文件删除。
- 本章验证后清理了生成缓存：
  - `tauri-client\dist`
  - `tauri-client\src-tauri\target`
  - `tauri-client\.tmp-ocr-processing-check`
  - `tauri-client\.tmp-normalization-debug`

### 本章不做

- 不引入默认 VLM。
- 不把千万术语做成硬编码词典。
- 不修改用户翻译服务私有配置。
- 不做发布签名、自动更新、CDN 或版本回滚。
- 不提交、不推送、不打 tag。

### 验证

- `npm run check:ocr-processing`：通过。
  - 新增覆盖长段英文正文合并。
  - 新增覆盖论坛列表不被正文段落规则误合并。
- `npm run check:ocr-fixtures`：通过。
  - 生成式中文、英文 UI、小字技术文本、韩文、日文、阿拉伯文 fixture 均通过。
  - `测试图片\1.png` 到 `测试图片\4.png` 均通过。
  - `测试图片\测试2\原始文本.png`：`45` blocks，约 `2435ms`。
  - `测试图片\测试2\微信翻译结果.png`：`6` blocks，约 `2017ms`。
  - `测试图片\测试2\我们的截图翻译结果.png`：`12` blocks，约 `2594ms`。
- 真实 normalization 抽查：
  - `测试图片\测试2\原始文本.png`：`45` raw blocks → `1` final paragraph。
  - `测试图片\测试2\微信翻译结果.png`：`6` raw blocks → `1` final paragraph。
  - `测试图片\4.png`：`18` raw blocks → `16` final forum/list blocks。
- `npm run build`：通过，Vite 仍只有 chunk 大于 `1200 kB` 的既有体积警告。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `cmd /c "build.bat --no-pause"`：通过，生成 `release\YSN-Screenshot-Translator\tauri-client.exe`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过，生成 `release\ScreenshotTranslator_Windows.zip`，约 `210.83 MB`。
- release smoke：`release\YSN-Screenshot-Translator\tauri-client.exe` 启动后保持存活。
- `npm run smoke:translate-service`：通过，公网 `https://ocr.yousn.me` 总计 15 blocks / 5 batches 约 `3890ms`。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`：通过。
- Windows 图标缓存刷新：已执行 `ie4uinit.exe -show` 和 `ie4uinit.exe -ClearIconCache`。

### 当前风险

- 段落 profile 是布局后处理优化，不会减少 OCR 引擎本身耗时；它解决的是长段翻译质量和 block 粒度，不是 detector/recognizer 的计算成本。
- `原始文本.png` 的 one-shot OCR 仍约 `2.4–3.0s`；常驻 worker 能省掉初始化，但字多时 detector/recognizer 仍是主要耗时。
- 任务栏图标在 Windows 上可能受资源管理器图标缓存影响；本章已刷新缓存，但极端情况下用户仍可能需要重启 Explorer 或换 exe 路径观察。
- Vite 主 chunk 仍约 `1.2MB`，目前只是警告；后续若继续加 UI/诊断模块，应该做按页 code split。

### 下一章建议

Chapter 146：继续拆 OCR 延迟构成。优先把常驻 worker 的 warm/cold、detector、recognizer、IPC、前端裁剪、normalization、翻译请求分别打点到诊断报告；再决定是否做 detector 复用、英文长段专用 resize 策略或更细的脚本早停。

## Chapter 146 - 段落翻译行级渲染、OCR 快路径与程序图标托盘统一（2026-06-03）

### 目标

- 修复 `测试图片\测试2\原始文本.png` 段落翻译后覆盖层出现超大字体的问题。
- 继续优化 RapidOCR `1200–2000ms` 热路径延迟，先做低风险的 worker 预热和截图快路径参数。
- 按用户要求把任务栏/托盘图标也换成程序图标。

### 实际完成

- 修复大字渲染根因：
  - 保留 Chapter 145 的段落级翻译块，继续让翻译模型拿到完整上下文。
  - 新增行级渲染分发：翻译请求用段落 block，覆盖绘制改回使用 paragraph 形成前的行级 `renderBlocks`。
  - 段落译文按原始行长度权重拆回原行锚点，避免用整段大框估算字体。
  - 新增门禁断言：段落翻译不能使用高达整段的 merged paragraph box 绘制。
- 优化 OCR 热路径：
  - RapidOCR runner 默认关闭截图横排文本不需要的方向分类：`use_cls=False`。
  - 检测放大边长从默认 `736` 调整为 `640`，保留环境变量 `YSN_RAPIDOCR_DET_LIMIT_SIDE_LEN` 可回退到 `736`。
  - 重新打包 `rapidocr-runner.exe`，并用真实 fixture 验证。
  - 截图页加载配置、进入翻译模式、执行翻译动作时后台预热 `prewarm_local_ocr_models`，避免第一次翻译才启动 worker。
  - `restoreCollapsedUiTextSpacing` 增加 `fort the` → `for the`、`P've` → `I've` 修复，弥补快路径下更容易出现的少量英文误拼。
- 图标统一：
  - `taskbar-16x16.png`、`taskbar-32x32.png`、`taskbar-64x64.png`、`taskbar.png`、`taskbar.ico` 已全部用 `程序图标.ico` 重新生成。
  - 保持 Rust 托盘入口使用 `taskbar-32x32.png`，但该文件现在与程序图标同源。

### 新增文件

- `tauri-client/src/translation-render/renderTranslationDistribution.ts`

### 修改文件

- `tauri-client/src-tauri/rapidocr/rapidocr_runner.py`
- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/rapidocr-runner.exe`
- `tauri-client/src-tauri/icons/taskbar-16x16.png`
- `tauri-client/src-tauri/icons/taskbar-32x32.png`
- `tauri-client/src-tauri/icons/taskbar-64x64.png`
- `tauri-client/src-tauri/icons/taskbar.png`
- `tauri-client/src-tauri/icons/taskbar.ico`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/ocr-processing/normalizationReport.ts`
- `tauri-client/src/ocr-processing/textSpacing.ts`
- `tauri-client/src/translation-render/index.ts`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `tauri-client/scripts/check-ocr-fixtures.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无源码删除。
- 验证后建议继续清理生成缓存：
  - `tauri-client\dist`
  - `tauri-client\src-tauri\target`
  - `tauri-client\.tmp-ocr-processing-check`
  - `tauri-client\.tmp-normalization-debug`

### 本章不做

- 不默认启用 VLM。
- 不恢复旧自研 `YSN OCR Runtime`。
- 不恢复 PaddleOCR-json 作为普通主路径。
- 不做发布签名、自动更新、CDN 或版本回滚。
- 不提交、不推送、不打 tag。

### 验证

- 段落 normalization 抽查：`测试图片\测试2\原始文本.png` 为 `41 raw blocks → 7 render blocks → 1 translation paragraph`。
- RapidOCR worker 热路径测速：
  - `测试图片\测试2\原始文本.png`：热 worker `smallTextRetry=false` 平均约 `865ms`，`smallTextRetry=true` 平均约 `1036ms`。
  - `测试图片\4.png`：热 worker `smallTextRetry=false` 平均约 `929ms`，`smallTextRetry=true` 平均约 `1140ms`。
  - `测试图片\3.png`：热 worker `smallTextRetry=false` 平均约 `651ms`，`smallTextRetry=true` 平均约 `828ms`。
  - 优化前同类热 worker 多在 `~900–1300ms`，本章低风险参数后小图和正文图已有可见下降；真实应用还会受截图裁剪、base64、IPC 和翻译链路影响。
- `npm run check:ocr-processing`：通过。
- `npm run check:ocr-fixtures`：通过。
  - `测试图片\测试2\原始文本.png`：`41` blocks，约 `2110ms` one-shot。
  - `测试图片\测试2\微信翻译结果.png`：`6` blocks，约 `1823ms` one-shot。
  - `测试图片\测试2\我们的截图翻译结果.png`：`7` blocks，约 `1960ms` one-shot。
- `npm run build`：通过，Vite 仍只有 chunk 大于 `1200 kB` 的既有体积警告。
- `cargo check`：通过。
- `cargo test`：通过，`17 passed; 0 failed`。
- `npm run build:rapidocr-runner`：通过，重新生成内置 runner。
- `cmd /c "build.bat --no-pause"`：通过。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过，zip 约 `210.83 MB`。
- release smoke：`release\YSN-Screenshot-Translator\tauri-client.exe` 启动后保持存活。
- `npm run smoke:translate-service`：通过，公网 `https://ocr.yousn.me` 总计 15 blocks / 5 batches 约 `2995ms`。
- Windows 图标缓存刷新：已执行 `ie4uinit.exe -show` 和 `ie4uinit.exe -ClearIconCache`。

### 当前风险

- RapidOCR 快路径把检测边长降到 `640`，真实小字/低清图如果回归，可用 `YSN_RAPIDOCR_DET_LIMIT_SIDE_LEN=736` 回退验证。
- `smallTextRetry=true` 仍会增加部分场景耗时；当前默认保留，因为它保护小字和技术文本召回，后续应根据图像特征动态关闭。
- 本章自动验证了行级渲染分发的几何逻辑，但仍需要真实覆盖层视觉 smoke 截图确认背景擦除、长译文换行和边界截断。
- 真实用户感知延迟仍包含截图裁剪、UIA 文本源命中、OCR、翻译网络、canvas 渲染；下一章应把这些细分写进诊断报告。

### 下一章建议

Chapter 147：做真实覆盖层视觉 smoke，重点复测用户刚才的大字截图；同时把 UIA 文本源命中率、OCR worker warm/cold、候选数、engine_ms、render 分发块数和翻译耗时一起写入复制诊断报告。

## Chapter 147 - 文本源选区防串台、译文渲染保险与透明图标闭环（2026-06-03）

### 目标

- 修复用户侧边栏样例里“只截单个单词/小区域，却翻出整列菜单并被放大很多倍”的问题。
- 防止 UIA 文本源抢跑把大容器、父节点、整页文本误当作当前选区 OCR block。
- 给翻译覆盖渲染增加字号、换行和裁剪保险，避免异常长译文撑爆小框。
- 将程序图标重新处理为透明底，替换应用图标、任务栏/托盘图标，并刷新 Windows 图标缓存。

### 根因

- Chapter 144 的 UIA 文本源快路径只判断元素矩形与选区是否“相交”，没有要求元素主体落在选区内。
- UIA 经常返回侧边栏、窗口、Group、Pane 等父容器文本；这些父容器只要碰到小选区，就会被旧逻辑裁成当前 crop 大小，导致整列菜单文本进入翻译。
- 截图页旧逻辑还把 canvas/CSS 选区坐标直接加到屏幕坐标上，没有统一使用已加载截图的物理像素选区；在缩放/DPI 场景下更容易错配。
- 渲染层旧逻辑最低字号为 `10px`，且没有对异常长译文做局部 clip，父容器文本误入后就表现为“单词也突然放大很多倍”。

### 实际完成

- 文本源防串台：
  - 新增纯函数模块 `textSourceSelection`，用物理像素选区与 UIA 屏幕坐标匹配。
  - 元素必须有足够 element coverage，过大的父容器、仅擦边相交元素、长聚合文本会被拒绝。
  - 对包含多个子元素的聚合容器做二次剔除，优先保留真实落在选区内的叶子文本。
  - 截图页 `getTextSourceBlocksForCurrentSelection` 改为返回命中数、拒绝数、聚合拒绝数和 coverage 诊断。
  - 如果文本源只剩聚合容器或命中质量不足，自动回落 RapidOCR，不再把错误 UIA 文本送去翻译。
- 翻译渲染保险：
  - 覆盖渲染最低字号从 `10px` 降到 `7px`，更适合小 UI 文本。
  - 渲染前按可绘制区域自动换行、逐步缩小字号。
  - 绘制时对擦除区域加 canvas clip，异常译文不会越界污染整张图。
- 透明图标：
  - 根目录 `程序图标.ico` 和 `app.ico` 已移除烘焙白底，保留透明 alpha。
  - Tauri 应用图标、Windows Square/Store 图标、任务栏/托盘图标全部由透明源重新生成。
  - `icon.icns` 也同步更新，避免跨平台资源仍带旧白底。
  - 已执行 Windows 图标缓存刷新。

### 新增文件

- `tauri-client/src/utils/textSourceSelection.ts`

### 修改文件

- `程序图标.ico`
- `app.ico`
- `tauri-client/src-tauri/icons/32x32.png`
- `tauri-client/src-tauri/icons/128x128.png`
- `tauri-client/src-tauri/icons/128x128@2x.png`
- `tauri-client/src-tauri/icons/icon.png`
- `tauri-client/src-tauri/icons/icon.ico`
- `tauri-client/src-tauri/icons/icon.icns`
- `tauri-client/src-tauri/icons/Square30x30Logo.png`
- `tauri-client/src-tauri/icons/Square44x44Logo.png`
- `tauri-client/src-tauri/icons/Square71x71Logo.png`
- `tauri-client/src-tauri/icons/Square89x89Logo.png`
- `tauri-client/src-tauri/icons/Square107x107Logo.png`
- `tauri-client/src-tauri/icons/Square142x142Logo.png`
- `tauri-client/src-tauri/icons/Square150x150Logo.png`
- `tauri-client/src-tauri/icons/Square284x284Logo.png`
- `tauri-client/src-tauri/icons/Square310x310Logo.png`
- `tauri-client/src-tauri/icons/StoreLogo.png`
- `tauri-client/src-tauri/icons/taskbar-16x16.png`
- `tauri-client/src-tauri/icons/taskbar-32x32.png`
- `tauri-client/src-tauri/icons/taskbar-64x64.png`
- `tauri-client/src-tauri/icons/taskbar.png`
- `tauri-client/src-tauri/icons/taskbar.ico`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/translation-render/renderTranslatedBlocks.ts`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无源码删除。
- 本章结束前继续清理临时预览图、前端 dist、Rust target 和临时检测目录。

### 本章不做

- 不默认启用 VLM。
- 不把 UIA 文本源当作强制主路径；它仍是快路径，质量不足必须回落 RapidOCR。
- 不继续扩大 OCR 模型路线，本章只修选区串台和渲染异常。
- 不提交、不推送、不打 tag。

### 验证

- `npm run check:ocr-processing`：通过。
  - 新增覆盖：小选区中存在侧边栏父容器文本时，只保留 `Dashboard` / `Operations overview` 子文本。
  - 新增覆盖：只有整列聚合文本时直接返回空 blocks，触发 RapidOCR 回落。
- `npm run build`：通过，Vite 仍只有 chunk 大于 `1200 kB` 的既有体积警告。
- `cargo check`：通过。
- `npm run check:ocr-fixtures`：通过。
  - `测试图片\4.png`：`18` blocks，约 `2748ms`。
  - `测试图片\测试2\原始文本.png`：`41` blocks，约 `2460ms` one-shot。
  - 韩文/阿拉伯文完整 fallback 仍较慢，分别约 `7753ms` / `9192ms`。
- `cmd /c "build.bat --no-pause"`：通过，生成 `release\YSN-Screenshot-Translator\tauri-client.exe`。
- release smoke：新版 `release\YSN-Screenshot-Translator\tauri-client.exe` 启动后保持存活。
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`：通过，生成 `release\ScreenshotTranslator_Windows.zip`，约 `210.82 MB`。
- 透明图标像素检查：
  - `app.ico` 透明像素 `62702 / 65536`，不再有不透明白底像素。
  - `tauri-client/src-tauri/icons/icon.png` 透明像素 `62702 / 65536`，不再有不透明白底像素。
  - `taskbar-32x32.png` 与 `32x32.png` 均不再有不透明白底像素。
- Windows 图标缓存刷新：已执行 `ie4uinit.exe -show`、`ie4uinit.exe -ClearIconCache` 并尝试清理 Explorer icon cache 文件。

### 当前风险

- UIA 文本源快路径现在更保守：部分复杂应用可能少命中文本源，但会自动回落 RapidOCR；这是为了优先保证翻译正确性。
- 本章用纯函数和 fixture 验证了串台根因，但真实覆盖层还需要用户用侧边栏样例再肉眼确认一次视觉效果。
- 韩文/阿拉伯文等非中英路径仍慢，这是多语言模型 fallback 成本，不属于本章修复范围。
- Windows 图标缓存有时被 Explorer 锁住；若用户仍看到旧白底图标，重启资源管理器或换 exe 路径可强制刷新。

### 下一章建议

Chapter 148：继续优化真实用户感知延迟。优先把 small-text retry 改成动态触发，并把 OCR worker warm/cold、detector、recognizer、文本源拒绝原因、翻译服务耗时合并到可复制诊断报告。

## Chapter 148 - 翻译批处理延迟、渲染字号保险与截图误触状态机修复（2026-06-03）

### 目标

- 深入审查 `server/app.py`、`server/translator.py` 的批量翻译、缓存和 LLM 超时路径，减少慢失败导致的长等待。
- 修复英文/段落翻译后覆盖层字体异常变大的剩余风险。
- 修复 `Alt+A` 截图入口、native pointer recovery、拖拽释放和确认入口之间的误触/自动确认风险。

### 实际完成

- 服务端翻译性能：
  - `TranslationCache` 从列表式 LRU 改为 `OrderedDict` LRU，生产缓存测试不再复制一份假实现，而是直接覆盖 `translator.py` 的真实缓存类。
  - Google/Baidu/DeepL/LLM timeout 改为可通过环境变量配置，并把默认值对齐客户端默认 9 秒翻译超时。
  - LLM 批量翻译新增总预算：批量网络失败时不再逐条慢 fallback；只有批量响应成功但缺少分隔段时，才在剩余预算内做精准补偿。
  - `/api/translate_text` 移除应用层串行单条 fallback，批处理崩溃时返回行数对齐的空译文，并在 `timings` 暴露 `provider_failures`、`provider_fallbacks`、`provider_batch_ms`、`provider_fallback_ms`。
  - 客户端本地翻译缓存查询优先使用预热 health 得到的真实服务端 channel，减少 `config.channel` 为空时重复文本错过本地 memory 的情况。
- 覆盖渲染字号：
  - `distributeTranslationsForRender` 在只匹配到一个原始 render line 时也使用原始行锚点，不再回退到高大的段落合并框。
  - `renderTranslatedBlocks` 的字号估算同时参考原框高度和原文宽度密度，限制稀疏高框/段落框把短译文放大到异常字号。
  - 翻译擦除区域最大横向扩展收紧，避免异常长译文擦掉整行或整屏。
  - `npm run check:ocr-processing` 增加段落高框字号和单行分发门禁。
- 截图交互状态机：
  - Rust 全局截图/翻译/录制快捷键增加 450ms 防抖，避免按键重复 Pressed 事件连续重启截图窗口。
  - Rust `screenshot-updated` payload 携带 mode，前端按 payload 初始化 session，避免 normal/translate/record 模式事件竞态串台。
  - native pointer recovery 改为先记录初始点，只有鼠标移动超过门槛才接管创建选区，避免主窗口按钮点击或残留左键状态误建选区。
  - 前端确认入口统一检查有效选区、非拖拽/缩放/标注绘制状态，以及 120ms 稳定时间；双击确认需要更长稳定时间，防止双击选择候选后直接复制。

### 新增文件

- 无。

### 修改文件

- `server/app.py`
- `server/translator.py`
- `server/tests/test_cache.py`
- `server/tests/test_translate_text.py`
- `server/tests/test_translator.py`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/src/translation-render/renderTranslatedBlocks.ts`
- `tauri-client/src/translation-render/renderTranslationDistribution.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`

### 删除文件

- 无。

### 本章不做

- 不默认启用 VLM。
- 不恢复旧自研 `YSN OCR Runtime`。
- 不恢复 PaddleOCR-json 作为普通主路径。
- 不做发布签名、自动更新、CDN 或完整 release 重新打包。
- 不提交、不推送、不打 tag。

### 验证

- `python -m pytest server\tests`：通过，`31 passed, 1 skipped`。
- `npm run check:ocr-processing`：通过。
- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。
- `cargo check`：通过。
- `cargo fmt`：已执行；无关 `text_source.rs` 格式 churn 已手动恢复。

### 当前风险

- 本章修复了服务端慢失败和状态机误触的根因路径，但仍建议用真实 `Alt+A`、主窗口按钮、翻译按钮、按住拖拽、多屏/DPI 场景做人工 smoke。
- Google 免费通道仍有质量和公网 RTT 风险；正式质量路线仍应优先配置 LAN/近端服务或付费 LLM/Baidu/DeepL 通道。
- 动态 small-text retry、OCR worker detector/recognizer 细分打点仍未完成，继续留给下一章。

### 下一章建议

Chapter 149：做真实覆盖层肉眼 smoke，重点复测用户遇到的英文大字与 Alt+A 误触；继续把 small-text retry 改成动态触发，并将 OCR worker warm/cold、detector、recognizer、UIA 文本源拒绝原因合并进可复制诊断报告。

## Chapter 149 - Alt+A 后 Ctrl+S 即时保存焦点修复（2026-06-05）

### 目标

- 修复用户反馈的 `Alt+A` 进入截图后，不能马上按 `Ctrl+S` 保存、需要额外点击一次的问题。
- 保留 Chapter 148 为防止误触加入的选区确认稳定时间，不重新放开误触风险。

### 实际完成

- 截图层 ready 后同时调用 Tauri 窗口 `setFocus()` 和 canvas `focus()`，并在 native `overlay_ready_to_show` 完成后再次聚焦。
- 鼠标开始框选、智能检测选区完成、手动框选释放后主动恢复截图窗口/画布焦点，保证随后的 `Ctrl+S` 能被截图页收到。
- 将过早触发的复制/保存确认从静默忽略改为短延迟执行：如果只是不满足 120ms 选区稳定时间，会自动等到安全窗口再继续保存。
- 新截图会话、取消、关闭和页面卸载时清理待执行确认，避免延迟保存泄漏到下一次截图。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改全局快捷键注册逻辑。
- 不调整截图选区确认的 120ms 防误触门槛。
- 不碰 OCR、翻译、录制、图标或发布配置。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。

### 当前风险

- 已通过 TypeScript/Vite 构建验证；真实 `Alt+A`、框选后立刻 `Ctrl+S` 仍建议在 Windows 桌面上做一次人工 smoke。

### 下一章建议

- 用真实桌面流程复测 `Alt+A -> 框选 -> 立即 Ctrl+S`、`Alt+A -> 智能点选 -> 立即 Ctrl+S`、`Ctrl+C` 和双击复制，确认焦点修复没有影响防误触状态机。

## Chapter 150 - 第二次录制控制条生命周期修复（2026-06-05）

### 目标

- 修复第一次录制后，第二次进入录制时只有蓝色区域框、没有底部控制条的问题。
- 防止控制条窗口创建失败时截图页仍把录制入口当作成功并隐藏自身。

### 实际完成

- `openRecordingWindows` 打开前不再只给旧控制条 250ms 的短超时，而是销毁旧 `recording_control` 后轮询确认 label 已释放。
- 新控制条窗口必须收到 `tauri://created` 才算创建成功；若收到 `tauri://error` 或超时，会关闭录制蓝框/残留窗口并把错误抛回截图页。
- 调整打开顺序：先创建并确认控制条，再显示原生录制蓝框，避免出现“蓝框已显示但控制条创建失败”的半成功状态。
- 仍保留原有 `recording-overlay-ready` / `recording-overlay-session` 会话发送机制，录制按钮、暂停、停止保存、打开目录和复制视频行为不变。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/utils/recordingWindows.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 FFmpeg 录制参数。
- 不改录制控制条 UI 排版。
- 不改截图、OCR、翻译或图标相关逻辑。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。

### 当前风险

- 已通过前端类型检查和构建；真实连续录制仍建议人工 smoke：录制一次关闭后，立刻再次 `Alt+A -> 框选 -> 录制`，确认控制条每次都出现。

### 下一章建议

- 补一轮真实录制流程验收：开始、暂停、继续、停止保存、红色关闭取消、保存后再开第二次、第三次，确认窗口生命周期和 FFmpeg 进程都能完整回收。

## Chapter 151 - 录制控制条动态会话 Label 修复（2026-06-05）

### 目标

- 修复上一章仍会报 `recording_control did not close in time`，导致录制入口无法再次打开的问题。
- 对齐用户预期：按 `Esc` 或关闭退出录制后，下一次进入录制应是新的控制条会话，不应被旧窗口 label 生命周期阻塞。

### 外部资料与同行模式

- Tauri v2 官方 `WebviewWindow` 文档说明 webview/window label 是唯一标识，`getByLabel` 会按 label 查找已有实例，动态窗口创建应监听 `tauri://created`。
- Tauri 官方文档也说明 `destroy()` 理论上会强制关闭窗口，不触发 `closeRequested`；但本项目真实行为显示固定 label 仍可能在短时间内无法释放。
- 上游 issue 中有开发者反馈动态 `WebviewWindow` 创建失败时需要监听 `tauri://error` 才能拿到真实错误。
- 同类多窗口/浮窗产品通常采用两种稳定模式：常驻单例 hide/show，或每次会话使用唯一窗口 ID；本项目录制入口更适合唯一会话 label。

### 实际完成

- `recording_control` 改为每次录制生成唯一 label：`recording_control_<timestamp>_<random>`。
- `main.tsx` 改为识别所有 `recording_control*` 窗口并加载录制控制条页面。
- 打开录制前仍尽力清理旧控制条，但不再因为旧固定 label 未释放而阻塞新会话。
- 控制条页的录屏排除逻辑改为使用当前窗口自身 label，避免动态控制条被录进视频。
- 保留 `tauri://created` / `tauri://error` 创建确认，控制条未创建成功时仍会清理蓝框并抛错。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/main.tsx`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/utils/recordingWindows.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 FFmpeg 参数。
- 不改录制控制条 UI。
- 不改 OCR、翻译、截图保存或图标资源。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。

### 当前风险

- 构建已通过；真实连续录制仍需用户在 Windows 桌面复测 `Alt+A -> 框选 -> 录制 -> Esc/关闭 -> 再次录制`。

### 下一章建议

- 如果真实复测仍有残留窗口，下一步应把录制控制条改成 Tauri/Rust 侧托管的显式 session registry，并提供 `close_recording_controls` 命令统一清理所有 `recording_control*` 窗口。

## Chapter 152 - 录制退出强制销毁与禁止白屏主窗口恢复（2026-06-05）

### 目标

- 修复动态录制控制条方案下，退出时控制条卡住、关闭不了的问题。
- 修复退出录制时自动弹出主窗口且主窗口白屏的问题。
- 保持用户语义：录制退出就是退出当前录制会话，不应额外拉起无关主窗口。

### 外部资料与同行模式

- Tauri v2 官方窗口 API 说明 `close()` 会走 close requested 流程，而 `destroy()` 是强制关闭窗口。
- Tauri 多窗口讨论中常见模式是：可复用工具窗用 hide/show；一次性动态工具窗退出时用强制销毁，避免 close requested 拦截造成循环。
- 对截图/录制类浮窗产品，退出录制控制条通常只清理本会话浮窗和录制进程，不自动恢复主配置窗口，除非用户明确从主窗口启动并期望返回。

### 实际完成

- 录制控制条 `closeOverlay` 和 `cancelRecording` 最终关闭从 `close()` 改为 `destroy()`，绕开 close requested 拦截循环。
- `dismissOverlay` 移除 `restoreMainWindow` 自动 show 主窗口逻辑，避免退出录制时拉起白屏主窗口。
- `openRecordingWindows` 不再检测/隐藏/恢复主窗口；控制条 payload 明确 `restoreMainWindow: false`。
- 保留动态控制条 label 和创建成功确认逻辑，控制条仍按当前自身 label 做录屏排除。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/utils/recordingWindows.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 FFmpeg 参数。
- 不改控制条 UI 排版。
- 不改截图、OCR、翻译或图标资源。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。

### 当前风险

- 构建已通过；当前桌面上如果仍有上一版运行产生的白屏主窗口或卡住控制条，需要重启应用进程后再验证新版行为。

### 下一章建议

- 增加 Rust 侧 `force_close_recording_controls` 命令，按 label 前缀统一关闭所有 `recording_control*` 窗口和 native recording overlay，用作用户可恢复路径和自动 smoke 清理入口。

## Chapter 153 - 录制准备态颜色与强制清理入口修复（2026-06-05）

### 目标

- 修复打开录制控制条时，未开始录制却先短暂出现红框再变蓝框的问题。
- 修复关闭录制后仍可能残留控制条、蓝框或白屏主窗口的问题。

### 外部资料与同行模式

- Tauri v2 官方 WebviewWindow/Window API 显示，动态窗口应监听 `tauri://created` / `tauri://error`，强制关闭应使用 `destroy()` 而不是普通 `close()`。
- Tauri 白屏相关 issue/讨论中常见建议是避免在工具浮窗生命周期中反复 show/hide 主 webview；截图/录屏类同行通常让浮窗和主窗生命周期分离。
- 录屏工具通用状态语义：准备态为蓝框，实际录制中才变红，暂停才变黄。

### 实际完成

- 截图页进入录制准备态时从 `recording` 改为 `ready`，避免未点击录制就进入红色状态。
- 截图画布渲染区分 `ready` 和 `recording`：准备态蓝色，真正录制中才红色。
- 新增 Rust 命令 `force_close_recording_controls`：
  - 隐藏 native recording overlay。
  - 强制 `destroy()` 所有 `recording_control*` 和 `recording_notice` 窗口。
  - 隐藏误弹出的 `main` 主窗口。
- 录制控制条关闭路径改为调用 Rust 统一清理命令。
- 打开新录制控制条前也调用统一清理命令，减少旧控制条/蓝框残留影响下一次录制。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/utils/recordingWindows.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 FFmpeg 参数。
- 不改控制条 UI 布局。
- 不改 OCR、翻译、截图保存或图标资源。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。
- `cargo check`：通过。

### 当前风险

- 代码和构建已通过；如果当前桌面仍显示上一版卡住的白屏主窗口，需要先彻底结束旧进程，再用新进程验证。

### 下一章建议

- 做真实 Windows smoke：连续三次 `Alt+A -> 框选 -> 录制准备 -> 关闭`，确认始终蓝框准备、关闭后无白屏主窗、再次进入控制条正常出现。

## Chapter 154 - 录制 Session 稳定传递、拖动手柄与 Tooltip UI 修复（2026-06-05）

### 目标

- 修复点击开始录制时报 `Recording session is not ready`，导致无法录制的问题。
- 修复控制条左右拖动手柄无法拖动的问题。
- 重新整理录制控制条 UI，避免按钮 hover 提示词遮挡按钮。

### 外部资料与同行模式

- Tauri v2 WebviewWindow 文档说明动态窗口创建依赖 `tauri://created` / `tauri://error`，跨窗口事件存在时序要求；稳定产品不应只依赖“新窗口 ready 后再 emit”传递关键 session。
- Tauri 窗口自定义文档建议拖动区域使用 `data-tauri-drag-region`，复杂场景可手动调用 `startDragging()`；如果子元素也要拖动，需要给子元素也标记拖动区域。
- Ant Design Tooltip 文档支持 `placement`、`mouseEnterDelay`、`getPopupContainer`、`overlayStyle`；紧凑工具条应让 tooltip 延迟出现、放到按钮外侧，并避免 tooltip 捕获鼠标事件。

### 实际完成

- `openRecordingWindows` 创建控制条前把 `RecordingWindowPayload` 写入 `localStorage`，并把 `recordingSessionKey` 放进控制条 URL。
- `RecordingControlPage` 启动后优先从 URL key 读取 session，事件通道保留为备份，避免用户点击按钮时 session 尚未到达。
- 增加 `sessionReady` 状态，session 未就绪时禁用录制/暂停按钮，并提示“录制准备中”。
- 重写 `RecordingControlHud`：
  - 左右手柄改为 `data-tauri-drag-region` + `startDragging()` 双路径。
  - 按钮显式 `no-drag`，避免拖动区域吞掉点击。
  - Tooltip 改为顶部、延迟出现、`pointerEvents: none`，降低遮挡按钮的概率。
  - 控制条窗口高度增加到 `96px`，给 tooltip 留透明空间。
  - 按钮尺寸和视觉状态重新统一为更稳定的 36px 控件。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src/components/recording/RecordingControlHud.tsx`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/utils/recordingWindows.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 FFmpeg 参数。
- 不改 OCR、翻译、截图保存或图标资源。
- 不提交、不推送、不打 tag。

### 验证

- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。
- `cargo check`：通过。
- `git diff --check`：通过，仅有工作区 CRLF 提示。

### 当前风险

- 已通过构建和静态检查；拖动、tooltip 遮挡和 session 到达仍需要在真实 Tauri 窗口中人工 smoke。

### 下一章建议

- 真实复测 `Alt+A -> 框选 -> 录制准备 -> 拖动控制条 -> 开始录制 -> 暂停/继续 -> 关闭`，确认 session、拖动、tooltip、颜色和窗口回收都闭合。

## Chapter 155 - FFmpeg 合并稳定性与录制控制条闭环修复（2026-06-05）

### 目标

- 继续收口用户指定的三个问题：
  - 控制条关闭按钮只关闭录制控制条，不触发无关主窗口白屏。
  - 修复 `ffmpeg failed to merge recording segments: exit code: 0xbebbb1b7` 类分段合并失败。
  - 修复控制条 UI 胶囊形态、拖动和 tooltip 遮挡问题。

### 外部资料与同行模式

- FFmpeg 官方 concat demuxer 文档要求文件列表使用 `file 'path'` 语法，Windows 绝对路径和特殊字符需要 `-safe 0` 与正确转义。
- FFmpeg FAQ/同行实践通常先尝试 concat demuxer `-c copy`；如果流参数或时间戳不稳定，再 fallback 到重新编码输出。
- Tauri 官方窗口 API 说明 `destroy()` 会绕过 close requested，用于强制关闭一次性动态工具窗；`close()` 会走关闭事件链。
- Ant Design Tooltip 文档支持 `autoAdjustOverflow`、`getPopupContainer`、`placement` 和延迟参数，紧凑工具条应给 tooltip 留出可视区域。

### 实际完成

- FFmpeg 停止/合并链路：
  - 前端正常停止录制段从最多等 `1100ms` 改为最多等 `10000ms`，避免还没写完 MP4 metadata 就开始合并。
  - Rust `stop_recording` 从 `800ms` 改为 `8000ms` 正常等待；正常停止超时或退出异常会报错，不再静默 kill 后继续合并。
  - 取消录制仍走快速 kill 路径，不影响用户取消体验。
  - 合并前不再静默跳过缺失/空分段；任一分段缺失或为空都会明确报错，避免“成功但丢内容”。
  - concat list 路径转义改为 FFmpeg file-list 语法，修复包含 `'` 或 Windows 反斜杠路径的风险。
  - 合并先尝试 concat demuxer `-c copy -movflags +faststart`。
  - copy 失败后自动 fallback 到 concat demuxer 重新编码：`libx264 + aac + yuv420p + faststart`。
  - FFmpeg 合并失败时保留 stderr tail，不再只返回 exit code，便于后续真实设备定位。
- 控制条 session/UI：
  - 控制条 session 使用 URL key + `localStorage` 传递，跨窗口事件保留为备份。
  - session 未就绪时禁用录制按钮并显示“录制准备中”。
  - 左右拖动手柄使用 `data-tauri-drag-region` + `startDragging()` 双路径。
  - 控制条按钮显式 `no-drag`，避免拖动区域吞掉点击。
  - Tooltip 放到顶部、延迟出现、禁用鼠标事件，并扩大控制条窗口透明高度到 `96px`。
  - 控制条视觉统一为 52px 高、无多余阴影的胶囊工具条。
- 关闭/清理：
  - Rust `force_close_recording_controls` 使用 `destroy()` 强制关闭所有 `recording_control*` 和 `recording_notice`，并隐藏 native 蓝框和误弹主窗口。

### 新增文件

- 无。

### 修改文件

- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/components/recording/RecordingControlHud.tsx`
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/utils/recordingWindows.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### 删除文件

- 无。

### 本章不做

- 不改 OCR、翻译、截图保存或图标资源。
- 不提交、不推送、不打 tag。

### 验证

- `cargo test`：通过，`19 passed`。
  - 新增覆盖：FFmpeg concat list 路径转义。
  - 新增覆盖：FFmpeg stderr tail 摘要保留失败上下文。
- `npm run build`：通过，仍只有既有 `1200 kB` chunk warning。

### 当前风险

- 已完成代码级和构建级验证；真实录制仍需在 Windows 桌面进行端到端 smoke，重点观察 FFmpeg 合并是否成功、控制条关闭是否仍弹主窗口、tooltip 是否完全可见。

### 下一章建议

- 真实验收三轮录制：不暂停直接保存、暂停/继续后保存、取消后再次录制；确认输出文件可播放且关闭后没有白屏主窗口或残留控制条。

### 阶段 2：重构建立单一窗口生命周期控制源并彻底消灭 YsnTrans 幽灵窗口 (2026-06-05)
- **目标**：合并重构中散落的 window_control.rs 与 window_lifecycle.rs，并在单一源中彻底修复录像控制条关闭后出现的 YsnTrans 幽灵白窗。
- **添加文件**：无
- **修改文件**：
  - 	auri-client/src-tauri/src/lib.rs
  - 	auri-client/src-tauri/src/window_lifecycle.rs
  - 	auri-client/src-tauri/src/recording_commands.rs
- **删除文件**：
  - 	auri-client/src-tauri/src/window_control.rs
- **非目标**：不修改 UI，不修改底层 FFmpeg/OCR 参数。
- **验证状态**：cargo check 与 
pm run build 均已通过。已将 Tauri .hide() 升级为原生的 win32::ShowWindow(hwnd, SW_HIDE)，消除 Windows 焦点回落时的不可见状态不同步问题。
- **下一步**：阶段 3（前端模块化，抽取 React Pages 中的巨石逻辑到 Hooks 中）。


## Chapter 156 - 前端与后端巨石源文件彻底拆分与优化 (2026-06-06)

### 目标
- 将前端唯一超标巨石文件 `ScreenshotPage.tsx`（1308 行）和后端 `lib.rs` 进行了深度的二次解耦和模块化拆分。
- 消除项目里所有行数超过 800 行的源文件，以提升代码架构的可读性和维护性。
- 修复拆分后后端单元测试的模块级可见性错误，确保测试通过。

### 新增文件
- `tauri-client/src/hooks/useScreenshotTextSource.ts`
- `tauri-client/src/hooks/useScreenshotActions.ts`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`

### 修改文件
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src-tauri/src/tests.rs`
- `tauri-client/src-tauri/src/diagnostics.rs`
- `tauri-client/src-tauri/src/recording_overlay.rs`

### 验证结果
- `npm run build` 前端打包 100% 成功，`ScreenshotPage.tsx` 自身行数减少至 728 行（已全面低于 800 行安全上限）。
- `cargo test` 19 个单元测试全部正常运行并通过（19 passed, 0 failed）。
- `npm run check:i18n` 多语言词典 566 个 key 完全对齐。
- `npm run tauri dev` 成功拉起调试客户端，各项前后台功能运转正常，界面响应无阻塞。

### 下一章建议
- 开始优化截图/翻译/OCR 耗时，将耗时细节分别打点暴露给用户诊断，并继续优化 OCR candidate 和 LLM 通道。


## Chapter 158 - Screenshot Overlay Lifecycle Isolation (2026-06-07)

### Goal

- Make screenshot capture a T0 independent capability: the main window is not special-cased and remains a normal capture target if it is visible.
- Prevent half-initialized transparent screenshot windows from appearing as ghost windows.
- Keep Alt+A responsiveness by only waiting for the screenshot bitmap, canvas, and pointer interaction readiness before showing the overlay.

### Added Files

- None.

### Modified Files

- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files

- None.

### Non-Goals

- Did not hide, minimize, exclude, or restore the main window during normal screenshots.
- Did not change OCR, translation, recording, pin-window, or save/copy behavior.
- Did not commit, push, create branches, or tag releases.

### Actual Changes

- The screenshot overlay now stays non-interactive while initializing and only becomes interactive when the screenshot image and canvas are ready.
- The screenshot page root now uses the existing `screenshot-root initializing/ready` CSS states so the ready state is explicit instead of implicit.
- The canvas disables pointer events until `overlayVisible` is true, avoiding accidental click-through or first-click activation confusion.
- The frontend now waits two animation frames after setting overlay state before invoking `overlay_ready_to_show`, giving React/DOM time to commit the ready canvas before the native window is shown and focused.
- `overlay_ready_to_show` now returns an error if the target screenshot overlay window is missing, allowing frontend cleanup instead of silently leaving a bad state.
- Screenshot image load timeout, image load failure, invalid screenshot data, and activation failure now route through one cleanup path that cancels the screenshot and attempts to show a concise error message.

### Validation

- `npm run build`: passed. Existing Vite warnings remain: large chunk warning and mixed static/dynamic import warning for `@tauri-apps/api/window.js`.
- `cargo check`: passed.

### Current Risks

- Needs real Windows desktop smoke testing with the packaged/dev app: Alt+A from desktop, Alt+A while main window is foreground, tray Screenshot Now, and repeated cancel/retry loops.
- Failure toast visibility may depend on whether the screenshot webview is visible at the exact failure moment; cleanup is prioritized over leaving a ghost window.

### Next Recommended Chapter

- Run real interaction smoke tests for screenshot lifecycle: verify first left-click starts selection, the main window is captured normally when visible, no ghost transparent window appears, and repeated Alt+A does not leave hidden overlay state.


## Chapter 159 - WGC-Class Screenshot Backend Migration (2026-06-07)

### Goal

- Fix the reproduced issue where visible Tauri/WebView2 content is captured as a white rectangle during screenshots.
- Preserve the product rule: screenshots should capture the visible screen as-is, including the main window when it is visible.
- Keep the screenshot overlay hidden until its bitmap/canvas is ready.

### Evidence

- Reviewed `C:/Users/ysn/Videos/Snow Shot/SnowShot_Video_2026-06-07_05-00-20.mp4`, `C:/Users/ysn/Videos/Snow Shot/SnowShot_Video_2026-06-07_05-38-06.mp4`, and `C:/Users/ysn/Videos/Snow Shot/SnowShot_Video_2026-06-07_06-01-26.mp4` through extracted contact sheets.
- The latest mouse-only video showed the main window content rendering normally before screenshot, then appearing white in the captured screenshot background.
- This points to the old `screenshots` crate/legacy capture path being unable to reliably capture WebView2/D3D-backed windows, rather than a hotkey, focus, or main-window visibility bug.

### Added Files

- None.

### Modified Files

- `tauri-client/src-tauri/Cargo.toml`
- `tauri-client/src-tauri/Cargo.lock`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files

- None.

### Non-Goals

- Did not hide, minimize, or exclude the main window during ordinary screenshots.
- Did not change OCR, translation, recording, copy, save, or pin behavior.
- Did not commit, push, create branches, or tag releases.

### Actual Changes

- Added `xcap` as the primary monitor capture backend. On Windows, this provides a more modern capture path suitable for WebView2/D3D-backed content than the previous legacy capture path.
- Added `capture_current_monitor_png` with primary `xcap` capture and legacy `screenshots` fallback, so capture still has a recovery path if the new backend fails.
- Updated the normal screenshot flow to use the shared capture helper before showing the overlay.
- Updated `quick_fullscreen_capture` to use the same capture helper, keeping fullscreen clipboard behavior aligned with the normal screenshot path.
- Screenshot start and explicit tray `Show Main Window` clear stale `main` capture exclusion as a safety measure, but no longer rely on hide/show heuristics.
- `overlay_ready_to_show` now returns an error if the target screenshot overlay is missing, allowing frontend cleanup instead of silently leaving a bad state.
- The screenshot page root now uses explicit `screenshot-root initializing/ready` states and disables canvas pointer events until the overlay is ready.

### Validation

- `cargo check`: passed after adding `xcap`.
- `npm run build`: passed. Existing Vite warnings remain: large chunk warning and mixed static/dynamic import warning for `@tauri-apps/api/window.js`.

### Current Risks

- Needs real Windows smoke testing after fully restarting the app: visible main window + mouse-triggered screenshot, tray-hidden app + tray screenshot, and screenshot after recording controls have been opened/closed.
- If white capture still reproduces, the next step is a native diagnostics command that writes both `xcap` and legacy captures to disk side-by-side for the same frame.

### Next Recommended Chapter

- Run a real desktop smoke loop and compare captured backgrounds with visible WebView2/Tauri windows, browsers, Explorer, and other hardware-accelerated apps.


## Chapter 160 - Root Cache Cleanup Utility (2026-06-07)

### Goal

- Add a root-level utility BAT for clearing safe project/system caches and refreshing Windows Explorer during screenshot/window troubleshooting.

### Added Files

- `clean_all_cache.bat`

### Modified Files

- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files

- None.

### Non-Goals

- Does not delete models, RapidOCR resources, user config, history, recordings, source files, or `node_modules` itself.
- Does not automatically run cleanup during build or app startup.

### Actual Changes

- Added conservative cleanup for Vite output/cache, selected Rust incremental/build caches, temporary analysis frame folders, user/system temp files, Explorer thumbnail/icon cache, and Windows Explorer restart.
- The script warns when not run as Administrator because Windows Temp and shell cache files may be skipped.
- The script pauses at the end and prints recommended next steps for restarting YsnTrans and retesting screenshot capture.

### Validation

- Static inspection only; the cleanup script was not executed to avoid disrupting the active session.

### Next Recommended Chapter

- Run `clean_all_cache.bat` manually as Administrator if the white-window capture state persists, then restart the app and retest visible-main-window screenshot capture.


## Chapter 161 - Hidden Main Window Screenshot Ghost Closure (2026-06-07)

### Goal

- Continue from `GHOST_WINDOW_FIX_HANDOFF.md` and fix the remaining scenario 4 failure: after the main panel is hidden with the top-right `X`, `Alt+A` screenshot close must not leave a real white `YsnTrans` shell on the desktop.
- Keep the five required user scenarios automated and inspect real desktop images, not just Win32/Tauri visibility fields.

### Added Files

- None in tracked source.
- Temporary validation evidence only: `.codex-analysis/desktop-smoke.ps1` and `.codex-analysis/ghost-smoke-*` screenshots/logs. These remain untracked and should not be submitted.

### Modified Files

- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files

- None.

### Non-Goals

- Did not change OCR, translation, recording, annotation, copy/save, pin-window, or release packaging behavior.
- Did not re-enable the unsafe `xcap` WGC feature inside the Tauri main process.
- Did not commit, push, create branches, or tag releases.

### Actual Changes

- Added a Windows-only hidden-main parking path for screenshot lifecycle:
  - When screenshot starts and `main` was already hidden, save its normal position, move the HWND to `-32000,-32000`, hide it with `SetWindowPos(... SWP_HIDEWINDOW | SWP_NOACTIVATE)`, and wait for DWM settle before capture.
  - Before closing a screenshot overlay while `main` was originally hidden, park `main` offscreen again, move foreground focus to a non-app top-level window or shell fallback, clear active/focus handles, then hide the overlay without activation.
  - When the user reopens the main panel via single-instance launch, tray menu, tray click, or `activate_webview_window`, restore the saved normal position before showing/focusing it.
- Split screenshot overlay hiding away from the generic `robust_hide_window` path:
  - Screenshot windows now use `hide_window_without_activation`, avoiding `ShowWindow(SW_HIDE)` as the primary close path for overlays.
  - `cancel_screenshot` uses the same focus handoff and non-activating hide path.
- Added `GetShellWindow` FFI for shell focus fallback.

### Validation

- Reproduced the original scenario 4 failure before the fix:
  - `.codex-analysis/ghost-smoke-20260607-163819/04_show_x_close_alt_a_close/desktop-after.png` visibly showed the white `YsnTrans` shell.
  - `afterWhiteRatio=0.9614`, while Win32 still reported `mainAfter.visible=false`.
- Verified scenario 4 after the fix:
  - `.codex-analysis/ghost-smoke-20260607-164741/04_show_x_close_alt_a_close/fullscreen_temp.png` was clean.
  - `.codex-analysis/ghost-smoke-20260607-164741/04_show_x_close_alt_a_close/desktop-after.png` was clean.
  - Summary: `tempWhiteRatio=0.01`, `afterWhiteRatio=0.01`.
- Ran the full required five-scenario automated desktop smoke:
  - Output directory: `.codex-analysis/ghost-smoke-20260607-165233`.
  - 1 hidden `Alt+A` close: bottom and after clean.
  - 2 visible main `Alt+A` close: bottom clean; after restored visible.
  - 3 visible main minimized `Alt+A` close: app log confirmed `was_visible=true was_minimized=true` and `keep-minimized`; before/after desktop images showed no restored main panel.
  - 4 `X` hidden main `Alt+A` close: bottom and after clean.
  - 5 taskbar/single-instance restore then `Alt+A` close: bottom clean; after restored visible.
- `cargo check`: passed.
- `cargo test`: passed, `19 passed`.
- `npm run build`: passed. Existing Vite warnings remain: mixed static/dynamic import for `@tauri-apps/api/window.js` and chunk size over `1200 kB`.
- `git diff --check`: passed with only existing CRLF conversion warnings for touched Rust files.

### Current Risks

- The smoke script is intentionally temporary and untracked; keep using the saved images for this chapter's evidence but do not submit `.codex-analysis`.
- The hidden-main parking strategy is Windows-specific and deliberately scoped to screenshot lifecycle when `main` is already hidden. Future refactors should keep position restore wired into every user-visible "show main" entry.

### Next Recommended Chapter

- Repeat the five-scenario smoke against a packaged/release build after the next release build is produced, then continue broader screenshot lifecycle smoke on multi-monitor/DPI setups.
