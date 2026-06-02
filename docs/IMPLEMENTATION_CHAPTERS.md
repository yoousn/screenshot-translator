# 商业级改造执行记录

> 本文档是唯一施工日志，但不再逐章保留超长全文。旧的 1–97 章已压缩为里程碑摘要；后续只记录“最近章节详情 + 当前交接状态”。主方向以 `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 为准。

## 当前交接状态（2026-06-02）

### 当前阶段

- 当前主线：`YSN OCR Runtime` 商业级闭环。
- 当前最新完整验证章节：Chapter 98。
- 当前正在推进章节：Chapter 99：真实 ONNX inference probe 与 decode/postprocess 接线。
- Chapter 98 当前状态：已完成并通过完整商业检查。
- 关键原则：`runtimeInferenceReady` 仍必须保持 `false`，直到真实 ONNX inference、decode、postprocess、self-test 全部通过。

### 当前已验证命令

- Chapter 97 完整商业检查已通过：`powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`。
- Chapter 98 定向验证已通过：
  - `cargo fmt`
  - `cargo test ysn_ocr_runtime_adapter --lib`：`11 passed; 0 failed`
  - `cargo test ysn_ocr_runtime --lib`：`13 passed; 0 failed`
  - `cargo check`

### 当前未完成事项

- Chapter 99 还未开始：下一步接真实 ONNX inference probe / decode / postprocess，不得把 `runtimeInferenceReady` 改成 true。
- ONNX session readiness 目前只覆盖：缺失模型、损坏模型、session metadata 成功路径的结构化结果。
- 仍未接真实 OCR inference → decode → postprocess → OCR self-test。
- 仍未接真实 managed CDN / 模型托管源 / 签名 source index。
- 仍未做完整 Windows 人工验收：Alt+A、OCR、翻译、录制、复制、保存、取消、打开目录。

### 当前工作树提醒

- 当前工作树有大量历史未提交改动和未跟踪文件，这是长期连续改造积累的结果。
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
