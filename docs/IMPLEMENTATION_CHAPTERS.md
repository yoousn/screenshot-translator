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


