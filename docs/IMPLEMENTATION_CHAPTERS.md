# 商业级改造执行记录

> 本文档是唯一施工日志，但不再逐章保留超长全文。旧的 1–97 章已压缩为里程碑摘要；后续只记录“最近章节详情 + 当前交接状态”。主方向以 `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 为准。

## 当前交接状态（2026-06-03）

### 当前阶段

- 当前主线：产品内置 `RapidOCR / ONNXRuntime` OCR 主路径。
- 当前最新完整验证章节：Chapter 134。
- 当前正在推进章节：Chapter 135：真实截图 RapidOCR fixture 与多语言 fallback 性能优化。
- Chapter 134 当前状态：已完成，控制台/识字模型页轻量化、录制保存后二次录制、大模型提示词配置、前端构建和服务端测试均通过。
- 关键原则：旧自研 `YSN OCR Runtime` 已废弃为非主路径；普通主流程只走 RapidOCR runner。OCR ready 必须由打包 runner、自测、fixture、真实 `Ctrl+D` 结果窗和翻译覆盖层共同证明。

### 当前已验证命令

- Chapter 134 定向验证已通过：
  - `npm run check:i18n`
  - `npm run check:ocr-processing`
  - `npm run build`
  - `python -m pytest server\tests\test_translator.py server\tests\test_server.py`：`16 passed, 2 skipped`
  - `python -m pytest server\tests`：`26 passed, 3 skipped`

### 当前未完成事项

- Chapter 135 还未开始：下一步把用户真实截图/网页截图固化成 RapidOCR fixture，并优化韩文/阿拉伯文完整 fallback 的耗时。
- 打包版 RapidOCR 资源目录当前约 `340.7 MB`；旧 onefile 遗留 `rapidocr-runner.exe` 已清理，构建脚本已防止复发。
- 仍未做完整 Windows 人工验收：Alt+A、OCR、翻译、录制、复制、保存、取消、打开目录。
- 仍需验证真实复杂背景下的覆盖层擦除、长译文换行和边界截断。

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
