# YSN 商业级闭环总计划

> 本文档是项目唯一长期主计划。所有长期方向、产品闭环、架构路线、执行规则、验收标准和无人连续开发策略都以本文档为准。`docs/IMPLEMENTATION_CHAPTERS.md` 只作为施工日志，记录每个代码章节实际完成了什么。

## 长期跟进约定

- 本项目长期只依赖“两份文档 + 根目录 `AGENTS.md` 规则”：主计划负责方向和验收，章节日志负责施工记录，`AGENTS.md` 负责所有代码与产品质量红线。
- 不再保留五六份分散计划；长期只保留这两份文档是为了减少上下文切换、避免计划互相矛盾，并让无人连续开发始终有唯一入口。
- 后续无人连续开发时，默认先读本文档的当前优先级，再读 `docs/IMPLEMENTATION_CHAPTERS.md` 的最后一章，然后继续下一章，不再临时新建散乱计划文档。
- 每完成一个可验证章节，必须同步更新章节日志；如果主路线、商业级标准、OCR 架构或发布策略发生变化，必须同步更新本文档。
- 临时调研材料、模型实验记录、下载链接、截图对比和一次性命令输出，必须合并进本文档、章节日志、代码内 manifest/测试，或在用完后删除。
- 当前真实判断：从“能用的个人工具”推进到“接近商业级可售卖版本”，连续无人打断开发预计仍需要约 1–2 周；达到更完整的商业级可售卖闭环，还需要持续真实设备测试、安装包/升级链路、模型托管、错误恢复、性能压测和视觉 polish。
- 长期执行不以“章节数量”作为完成标准，只以用户主流程、商业级验收、发布闭环和真实设备结果作为完成标准。

## 0. 文档与执行规则

### 0.1 长期文档只保留两份

- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`：唯一主计划，负责方向、规则、架构、优先级、验收定义和闭环路线。
- `docs/IMPLEMENTATION_CHAPTERS.md`：唯一施工日志，负责记录每个章节的目标、改动文件、验证结果、遗留问题和下一章入口。
- 保留两份而不是一份的原因：主计划要保持可读、可决策、可验收；章节日志会持续增长，适合记录详细施工历史、验证命令和每章边界。如果全部塞进一份文档，主计划会很快被日志淹没，后续恢复工作反而更慢。
- 不再单独保留零散长期 Markdown，例如 OCR 临时计划、录制临时计划、模型下载临时计划、UI 临时计划。

### 0.2 新文档准入规则

只有满足以下任一条件，才允许新增长期文档：

- 主计划会因内容过大而明显降低可读性，并且新文档有清晰边界。
- 内容是稳定规范，例如协议、模型 manifest 格式、发布流程规范，且需要被代码或团队长期引用。
- 新文档必须先在本文档建立索引、说明用途和保留理由；否则视为临时文件，完成后必须删除或合并。

### 0.3 章节记录规则

每个可描述的代码章节完成后，必须追加到 `docs/IMPLEMENTATION_CHAPTERS.md`，格式至少包含：

- 章节编号和标题。
- 本章目标。
- 新增文件。
- 修改文件。
- 删除文件，如有。
- 本章不做。
- 验证命令和结果。
- 下一章建议。

### 0.4 无人连续执行规则

当用户离开或没有继续细化指令时，默认继续按本文档推进，不等待零散确认。执行顺序：

1. 先恢复工程健康，避免半成品继续扩大。
2. 再闭合一个完整用户主流程，而不是做一堆只完成一半的功能。
3. 再做大文件拆分、i18n、错误恢复、诊断、视觉 polish 和发布闭环。
4. 每章只解决一个清晰主题，避免跨太多领域导致回归风险。
5. 如果发现当前实现路线和商业级目标冲突，先修订本文档，再继续写代码。
6. 不为了快速落地选择会限制产品质量、扩展性或所有权的方案。
7. 不能真实验证时，产品状态必须显示“未就绪 / 待配置 / 待验证”，禁止假装 ready。
8. 每轮开始必须先确认 `git status` 和最新章节，避免覆盖已有半成品或重复实现。
9. 每轮结束必须留下可恢复现场：验证结果、遗留风险、下一章入口，全部写入章节日志。
10. 不提交、不推送、不打 tag，除非用户在当前上下文再次明确要求。
11. 如果连续开发遇到重大架构分歧，优先选择商业级长期路线，并把取舍写进本文档风险登记。

### 0.5 代码组织规则

- 一个独立组件、hook、service、utility、adapter、domain model 尽量单独一个文件。
- 页面文件只负责编排布局和数据流，不堆卡片、弹窗、工具栏、API 调用和复杂 helper。
- 大文件是设计异味；接近混合职责时就拆，不等到上千行。
- 新功能从一开始就按可扩展目录设计，避免后期为了继续开发被迫大拆。
- 修改大文件时，若能低风险抽出清晰模块，应顺手拆出。

## 1. 产品目标

把当前项目改造成接近商业级可售卖，并最终达到商业级可售卖标准的 Windows 桌面截图生产力应用。

### 1.1 核心能力

- 截图、标注、复制、保存、钉图、滚动截图。
- OCR 识字，源语言自动识别，不让用户手动选择源语言。
- 截图翻译，目标语言可选，默认简体中文。
- 视频录制，Snow Shot 风格区域录制控制条，自动保存到 `Videos\YSN`。
- 现代化配置中心，普通用户看到简单状态和主按钮，高级能力渐进展开。
- 诊断、自测、恢复、模型管理、发布、升级和用户错误路径闭环。

### 1.2 商业级定义

商业级不是“能用”，而是至少满足：

- 主流程稳定：截图、OCR、翻译、录制、保存、复制、打开目录、取消恢复都可靠。
- 性能可接受：启动、框选、OCR、翻译、录制控制不明显卡顿。
- 错误可恢复：依赖缺失、模型缺失、下载失败、权限失败、录制失败都有明确提示和下一步。
- 视觉有质感：界面现代、紧凑、低噪音，不像临时工具。
- 可维护：模块清晰，组件/服务/命令拆分合理，后续可继续扩展。
- 可发布：安装包、签名、升级、模型托管、版本回滚、诊断报告形成闭环。

## 2. 产品闭环路线

### P0：工程健康与可构建

- `check_commercial.ps1` 是当前统一商业检查入口。
- 前端 `npm run check:i18n`、`npm run check:ocr-processing`、`npm run build` 必须保持通过。
- Rust `cargo check`、`cargo test` 必须保持通过。
- release build / smoke launch 作为发布前门禁，不一定每章都跑。

### P1：截图主流程商业级闭环

- `Alt+A` 进入截图/框选。
- 截图态支持 `Ctrl+C` 复制、`Ctrl+S` 保存、`Ctrl+D` OCR、`Ctrl+Q` 翻译。
- OCR 结果窗口的主按钮应为“复制并关闭”。
- 截图、OCR、翻译失败时要有明确错误和恢复路径。

### P2：Snow Shot 风格录制闭环

- `Alt+A` 框选后点击录制，退出普通截图工具栏，只保留蓝色区域框和底部控制条。
- 蓝色为准备态，红色为录制中，黄色为暂停中。
- 控制条顺序：六点拖动｜录制/停止｜暂停/继续｜时间｜声音/麦克风｜分隔线｜文件夹｜红色关闭｜复制｜六点拖动。
- 停止后自动保存到系统视频目录下 `YSN`，不弹保存框。
- 文件夹按钮打开目录，复制按钮优先复制视频文件，失败则复制路径。
- 关闭按钮准备态退出，录制态取消并清理临时文件，保存后关闭控制条。

### P3：YSN OCR Runtime 主线闭环

长期方向不是外部 `.exe`，而是产品自有 `YSN OCR Runtime`：

- 集成 ONNX Runtime。
- 使用 managed model packs。
- 自动源语言/脚本检测。
- 多模型 OCR 池，不假设一个识别模型覆盖所有脚本。
- 按脚本、置信度、fallback scoring 路由。
- 支持低置信度重试和未来 VLM OCR fallback。
- `runtimeInferenceReady` 只能在真实 ONNX inference、decode、postprocess、self-test 全部通过后变为 true。

### P4：翻译质量闭环

- OCR 后处理过滤图标误识别的 `O/○/•`。
- 恢复英文单词空格，解决 `AddthemissingPATH...` 之类问题。
- 按同一行合并成虚拟行后再翻译。
- Prompt 要保留行数、翻译短 UI 文案、不要漏译英文单词。
- 技术词保护：`PATH`、`Windows`、`OCR`、`ONNX`、`RapidOCR`、`PaddleOCR-json`、`.exe`、命令参数、路径、包名。
- 回填渲染左对齐，尽量匹配原行高度、颜色和背景，减少突兀灰块。

### P5：配置中心闭环

- 页面定位为“识字模型 / 视频录制”配置中心。
- 普通用户看到状态卡、主操作、问题恢复。
- 高级用户可展开模型、source index、FFmpeg、兼容 OCR、诊断信息。
- 默认 OCR 主线是 YSN OCR Runtime / ONNX；PaddleOCR-json 仅为兼容模式。
- 视频区域保留 FFmpeg 检测、下载/选择、音频设备检测和默认保存目录说明。

### P6：发布、安装、升级闭环

- 形成可重复 release build。
- 发布前跑商业检查、release build、smoke launch。
- 明确模型下载源、SHA256、size、license、版本、回滚策略。
- 后续考虑签名、自动更新、崩溃/诊断报告、卸载清理、Windows 权限问题。

### P7：商业级 polish 与真实测试

- 建立真实 Windows 人工验收清单。
- 用真实截图、GitHub 英文列表、多语言 UI、技术文本测试 OCR/翻译。
- 用真实录制场景测试开始、暂停、继续、停止、取消、复制、打开目录。
- 处理高 DPI、多屏、缩放、权限、音频设备缺失、路径含中文等场景。

## 3. OCR 模型策略

### 3.1 为什么不是只用轻量 RapidOCR

- Snow Shot 使用轻量 ONNX 包可以作为参考，但商业级多语言、自动源语言、翻译质量和可维护性不能只依赖一个轻量模型。
- 轻量模型适合作为快速、低资源路径；完整产品需要多模型池、脚本路由、置信度 fallback、字典/后处理和 self-test。
- 我们可以吸收 RapidOCR 的 ONNX 工程化经验，但长期主线应是自有 runtime + managed model packs。

### 3.2 推荐路线

- 第一阶段：建立 manifest、source index、download plan、dictionary artifact、runtime adapter、decode/postprocess、自测门禁。
- 第二阶段：接入真实 managed source fixture，完成 model/dictionary 下载、校验、加载和失败恢复。
- 第三阶段：接入真实 ONNX inference，并把 detector、classifier、recognizer 的输出接到 decode/postprocess。
- 第四阶段：建立多脚本模型池和 fallback scoring。
- 第五阶段：真实多语言 OCR/翻译样例验收。

### 3.3 语言要求

- 源语言必须自动识别，不提供手动源语言选择作为普通主流程。
- 目标语言用户可选，默认简体中文。
- 基线支持：简体中文、繁体中文、英语、法语、日文、德语、西班牙语、葡萄牙语、意大利语、韩语、俄语、阿拉伯语、泰语、土耳其语，并保留未来扩展空间。

## 4. 默认产品决策

| 项目 | 默认值 |
|---|---|
| 默认 OCR | YSN OCR Runtime / ONNX Runtime 主线 |
| 备用 OCR | PaddleOCR-json 兼容模式 |
| 源语言 | 自动识别 |
| 默认目标语言 | 简体中文 |
| 录制 FPS | 30 FPS |
| 录制分辨率 | 1080p |
| 倒计时 | 默认 0s |
| 视频保存目录 | 系统视频目录下 `YSN` |
| 控制条风格 | 现代化半透明胶囊、圆角、轻阴影、左右六点拖动 |

## 5. 验收总清单

### 5.1 构建验收

- `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1` 通过。
- 发布前额外跑 `check_commercial.ps1 -TauriBuild -SmokeLaunch`。
- 不允许通过 UI 文案把未验证功能显示成 ready。

### 5.2 用户主流程验收

- `Alt+A` 框选、复制、保存、OCR、翻译全部能用。
- 录制从框选到准备态、开始、暂停、继续、停止保存、打开目录、复制、取消清理全部闭环。
- OCR 默认使用自有 runtime 主线；没有模型时清楚显示未就绪并给出恢复路径。
- 翻译对真实 GitHub 英文列表、技术文本、多语言 UI 文案达到接近 Snow Shot 的质量。

### 5.3 商业级验收

- 主流程稳定、错误可恢复、界面现代、配置简洁。
- 模型源可托管、可校验、可更新、可回滚。
- 诊断报告能帮助定位 OCR、录制、翻译、依赖、权限问题。
- 真实 Windows 设备验收有记录，有失败项就继续修。

## 6. 当前风险登记（2026-06-02，Chapter 98 后）

> 本节用于无人连续开发时快速判断“哪些已经真实验证，哪些仍然不能对用户承诺”。如果实现或验证状态变化，必须同步更新本节，禁止用 UI 文案假装能力已完成。

### 6.1 已验证事实

- 长期文档已收敛为两份：`docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 负责方向、验收和闭环，`docs/IMPLEMENTATION_CHAPTERS.md` 负责施工日志；Chapter 93 已明确不再保留五六份分散计划。
- 根目录 `AGENTS.md` 已写入商业级产品标准、OCR 战略方向、UI/UX 标准、代码组织标准、无人执行标准和实现质量红线。
- 商业级检查入口已统一为根目录 `check_commercial.ps1`。
- `npm run check:i18n` 已作为 i18n 门禁，当前 `446 zh-CN keys match 446 en-US keys`。
- `npm run check:ocr-processing` 已作为 OCR/翻译处理链路门禁。
- `npm run build` 前端生产构建已通过。
- `cargo check` Rust 检查已通过。
- `cargo test` Rust 测试已通过，当前商业检查结果为 `91 passed; 0 failed`。
- OCR Runtime 已具备 managed source、model pack、active model health、readiness steps、routing plan、preprocess、ONNX adapter scaffold、decode/postprocess、dictionary artifact contract、dictionary loader、dictionary download plan、model/dictionary artifact download activation、model/dictionary active health、配置中心 artifact 展示，Chapter 94 managed source index 对 model/dictionary artifact 元数据的导入、模板和 dry-run 支持，以及 Chapter 95 本地 managed source fixture 从 dry-run download plan 到 SHA 校验与 active artifact 激活的 smoke 覆盖，以及 Chapter 96 下载器 artifact size 校验与失败清理，以及 Chapter 97 active artifact size/SHA issue 进入 self-test/readiness blocker，以及 Chapter 98 ONNX session readiness probe 缺失/损坏模型结构化 blocker，但仍不能视为真实生产 OCR。
- Snow Shot 风格录制控制条、自动保存、边框状态色、打开目录、复制视频和取消清理已有核心实现与测试基础。

### 6.2 当前高风险区

| 风险 | 当前真实状态 | 下一步处理 |
|---|---|---|
| OCR Runtime 未达到生产 ready | 已有 ONNX Runtime scaffold、source/manifest/model pack/readiness 管理、schema contract、managed source publish layout、source index dry-run、dictionary artifact 元数据导入、本地 fixture 激活 smoke、SHA256/size 校验、active artifact issue readiness blocker、ONNX session readiness probe 错误路径，但真实模型输出到 decode 的业务执行、多模型 fallback 和完整端到端 self-test 仍未闭环 | 接入真实 managed source 中的模型与字典 artifact 下载/校验/加载到生产 pipeline、真实输出到 bridge 的端到端自测样例和低置信度 fallback，保持 `runtimeInferenceReady=false` 直到端到端通过 |
| 模型源托管未闭环 | 本地已有 managed source index 规则，Chapter 94 已支持 model/dictionary artifact 元数据导入，Chapter 95 已验证本地 fixture 的 SHA 校验和 active artifact 激活，Chapter 96 已补上下载器 size 校验，Chapter 97 已让 size/SHA active artifact issue 阻断 readiness；仍缺少 YSN-controlled model CDN、真实模型/字典 artifact、真实 SHA256、size、license 元数据和签名发布流程 | 基于已固化 publish layout 接入真实托管源、签名 index、下载校验、版本升级和回滚策略 |
| 真实 Windows 人工验收不足 | release smoke 只能证明进程启动，不能证明 Alt+A、OCR、翻译、录制、复制、保存等桌面交互全部正确 | 建立人工验收清单和可复现样例，逐项记录通过/失败/截图证据 |
| 翻译质量仍需样例验收 | OCR/翻译处理门禁已有合成 fixture，但用户的 GitHub 英文列表、多语言截图、技术文本仍需真实样例验证 | 补充 OCR blocks fixture、截图样例、translation payload fixture 和重绘对比验收 |
| 发布商业闭环未完成 | 安装包和 smoke launch 曾通过，但签名、自动更新、模型托管、错误恢复、真实设备矩阵仍未完整 | 建立发布清单、模型源发布流程、真实设备验收和版本回滚流程 |

### 6.3 下一轮优先级

1. Chapter 99：真实 ONNX inference probe 与 decode/postprocess 接线。
2. 打通真实 OCR Runtime 端到端：真实模型源、下载校验、ONNX session、preprocess、decode、postprocess、fallback 和 self-test。
3. 补齐 OCR/翻译质量 fixture：真实 GitHub 列表、技术文本、多语言 UI 文案、translation payload 与重绘对比。
4. 建立 Windows 人工验收清单：截图、OCR、翻译、录制、复制、保存、取消、打开目录和异常恢复。
5. 发布闭环：release build、smoke launch、安装包、模型托管、签名、升级、回滚。

## 7. 当前原则总结

- 商业级优先，不做只为快速落地的短期架构。
- OCR 是战略能力，长期主线是自有 ONNX Runtime + managed model packs。
- 源语言自动识别是硬要求，目标语言默认简体中文并可选。
- 任何 ready 状态必须由真实端到端验证证明。
- 文档只保留主计划和章节日志，代码每章结束必须留下可恢复现场。




