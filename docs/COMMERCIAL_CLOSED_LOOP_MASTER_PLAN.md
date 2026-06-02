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

### P3：RapidOCR / ONNXRuntime 主线闭环

当前方向已按用户决策从自研 `YSN OCR Runtime` 切换为产品自有打包的 `RapidOCR / ONNXRuntime` 主线：

- 默认使用 RapidOCR PP-OCRv5，保留 PP-OCRv4 作为高级可选模型版本。
- 打包 `rapidocr-runner` 作为产品内置 sidecar，不依赖用户手动安装 PaddleOCR-json 或 Python 环境。
- 自动源语言/脚本检测，不让普通用户手动选择源语言。
- 多识别模型池：中文/英文/韩文/阿拉伯文/俄文/泰文等按质量评分 fallback。
- 按脚本字符、置信度、噪声 block、fallback scoring 路由，避免单个假阳性 block 赢过真实多行文本。
- 支持低置信度重试和未来 VLM OCR fallback。
- OCR ready 只能在 RapidOCR runner、模型资产、postprocess、fixture smoke 和真实截图工作流全部通过后展示。

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
- 高级用户可展开 RapidOCR 模型版本、runner 状态、FFmpeg、诊断信息。
- 默认 OCR 主线是 RapidOCR PP-OCRv5 / ONNXRuntime；PaddleOCR-json 和旧自研 YSN OCR Runtime 不再作为主路径。
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

### 3.1 为什么切到 RapidOCR 主线

- 当前自研 OCR Runtime 在真实清晰截图上仍出现漏识别、小字错识别和英文长句不稳定，继续补底层 detector/recognizer 不是当前商业化最优路径。
- RapidOCR 已经工程化封装 ONNXRuntime、PP-OCRv5/V4 detector/classifier/recognizer 和多语言识别模型，更适合作为当前产品主路径。
- 产品仍保留所有权：runner、模型资产、候选评分、后处理、翻译保护、配置面板和自测门禁都由项目集成和控制，而不是把用户暴露给外部 OCR exe。
- 商业级重点转为：更好的脚本路由、候选评分、真实截图 fixture、翻译质量、覆盖层还原和发布体积控制。

### 3.2 推荐路线

- 第一阶段：打包 RapidOCR runner，默认 PP-OCRv5，提供 PP-OCRv4 高级选择。
- 第二阶段：建立固定生成式 fixture 和真实截图 fixture，覆盖中文、英文、小字技术文本、日文、韩文、阿拉伯文、混排 UI。
- 第三阶段：把候选评分、噪声过滤、技术文本清理、RTL/LTR 位置策略纳入门禁。
- 第四阶段：优化性能：常见中英文 UI 走快速路径，多脚本低置信度才进入完整 fallback。
- 第五阶段：把 OCR fixture、翻译 smoke、覆盖层视觉回归和真实 Windows 手工验收串进发布门禁。

### 3.3 语言要求

- 源语言必须自动识别，不提供手动源语言选择作为普通主流程。
- 目标语言用户可选，默认简体中文。
- 基线支持：简体中文、繁体中文、英语、法语、日文、德语、西班牙语、葡萄牙语、意大利语、韩语、俄语、阿拉伯语、泰语、土耳其语，并保留未来扩展空间。

## 4. 默认产品决策

| 项目 | 默认值 |
|---|---|
| 默认 OCR | RapidOCR PP-OCRv5 / ONNXRuntime 主线 |
| 备用 OCR | RapidOCR PP-OCRv4 高级可选；低置信度 fallback 后续接入 VLM OCR |
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
- OCR 默认使用内置 RapidOCR runner；runner 或模型缺失时清楚显示未就绪并给出恢复路径。
- 翻译对真实 GitHub 英文列表、技术文本、多语言 UI 文案达到接近 Snow Shot 的质量。

### 5.3 商业级验收

- 主流程稳定、错误可恢复、界面现代、配置简洁。
- 模型源可托管、可校验、可更新、可回滚。
- 诊断报告能帮助定位 OCR、录制、翻译、依赖、权限问题。
- 真实 Windows 设备验收有记录，有失败项就继续修。

## 6. 当前风险登记（2026-06-03，Chapter 134 后）

> 本节用于无人连续开发时快速判断“哪些已经真实验证，哪些仍然不能对用户承诺”。如果实现或验证状态变化，必须同步更新本节，禁止用 UI 文案假装能力已完成。

### 6.1 已验证事实

- 长期文档已收敛为两份：`docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 负责方向、验收和闭环，`docs/IMPLEMENTATION_CHAPTERS.md` 负责施工日志；Chapter 93 已明确不再保留五六份分散计划。
- 根目录 `AGENTS.md` 已写入商业级产品标准、OCR 战略方向、UI/UX 标准、代码组织标准、无人执行标准和实现质量红线。
- 商业级检查入口已统一为根目录 `check_commercial.ps1`。
- `npm run check:i18n` 已作为 i18n 门禁，当前 `520 zh-CN keys match 520 en-US keys`。
- `npm run check:ocr-processing` 已作为 OCR/翻译处理链路门禁。
- `npm run build` 前端生产构建已通过。
- `cargo check` Rust 检查已通过。
- `cargo test` Rust 测试已通过，当前本地结果为 `17 passed; 0 failed`。
- Chapter 133 已按用户决策把 OCR 主路径从自研 `YSN OCR Runtime` 切到产品内置 RapidOCR runner：默认 PP-OCRv5，PP-OCRv4 高级可选，Rust `run_local_ocr` 调用 RapidOCR JSON runner，配置中心改为 RapidOCR 状态面板，旧自研 Rust OCR 模块和旧模型包面板已删除。
- RapidOCR runner 已完成本地打包：`tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/rapidocr-runner.exe`，预热 V5/V4 多语言模型；清理旧 onefile 遗留 exe 后资源目录约 `340.7 MB`。
- Chapter 110 已在新电脑完成 LFS 模型落盘、release build、smoke launch、active 字典 LF/SHA 修复、运行时 OCR 热路径提速，以及真实桌面截图 crop 的 Rust OCR smoke；样例从约 `6.6s` 降到约 `2.0s`，但小字和英文长句准确率仍不足。
- Chapter 111 已补上截图翻译质量门禁：普通英文 UI 文案返回原文/空译文时不再静默成功，技术标识保留会明确提示；覆盖重绘不再擦除重画未变化文本，小选区按原文字边界锚定并限制溢出。
- Chapter 115 已确认家里内网翻译服务地址为 `http://192.168.1.3:8318`，本机配置已启用内网优先、公网 `https://ocr.yousn.me` 回落；未缓存单句内网约 `465ms`，公网约 `1029ms`。
- Chapter 116 已加入客户端短 UI 词典、持久化翻译缓存、技术文本本地保留、服务端 `timings` 字段和 smoke timing 输出；重复 UI 文本可在客户端或服务端缓存中毫秒级命中。
- Chapter 117 已在设置页加入本机翻译缓存状态、刷新和清理按钮，为缓存错译提供用户可见恢复路径。
- Chapter 118 已将服务端 `timings`、多行 block 保护和短 UI 词典部署到 N100；线上 `/api/translate_text` 已返回 `total_ms/provider_ms/cache_hits/provider_misses`。
- Chapter 119 已新增并验证 `deploy_n100_translation_server.ps1`，N100 部署流程包含备份、上传、语法检查、重启、health 和 timing smoke。
- Chapter 120 已修复 Latin 多语种源语言路由，法语/西语不再被强行按英文翻译；smoke 已要求法语/西语包含核心语义关键词，N100 已部署。
- Chapter 121 已把短 UI 词典抽成 `translationGlossary.json` manifest，前端和服务端共同使用，部署脚本会同步到 N100。
- Chapter 122 已让 `/api/health` 和 `/api/config/current` 暴露 `translation` metadata，包括术语表版本、加载状态、term 数和质量 flags；前端顶部服务状态 tooltip 会显示 channel、glossary version 和 Google 免费通道质量风险。
- Chapter 123 已将翻译 metadata 纳入“复制诊断报告”，包括服务 URL、channel、目标语言、本地术语表版本、本机缓存状态、服务端 health 和质量 warnings。
- Chapter 124 已在设置页对 Google 免费通道显示非阻断质量风险提示，建议正式多语言质量场景配置 Baidu 或 New API。
- Chapter 125 已在设置页加入翻译通道健康摘要，显示 Google/Baidu/New API 的配置完整性、最近测试状态、服务端当前通道和 Google 质量风险；设置保存、通道测试和 New API 模型拉取均已改成内网优先候选 URL。
- Chapter 126 已建立固定 OCR crop fixture 门禁：PowerShell 生成英文 UI、中文大字、小字技术文本三张 PNG，Rust ignored test 跑真实 ONNX OCR pipeline 并断言核心关键词；根目录商业检查新增可选 `-OcrFixtures`。
- Chapter 127 已加入 Latin/技术文本 OCR 异常评分，真实 OCR pipeline 和固定 fixture smoke 会输出 `latin-probable-tail-truncation`、`technical-path-extra-space`、`technical-path-digit-tail`、`technical-command-flag-tail` 等 issue code。
- Chapter 128 已把 quality issue 接入低风险 Latin 二次识别：命中异常的 Latin 行会尝试更大 recognition width 和更宽 crop；固定英文 UI fixture 已从 `savin/tex` 修复为 `saving/text`，固定技术小字 fixture 已从 `System3:/--hel]` 修复为 `System32/--help`，并补了窄范围 Windows 路径空格修复。
- Chapter 129 已把真实浏览器截图纳入 OCR fixture 门禁：用 Vite 应用真实页面截图作为调用方提供的 PNG，`check-ocr-fixtures.ps1` 支持 `-RealScreenshotPath` / `-RealExpectContains` / `-RealMinBlocks`，真实样例断言 `YSN`、`OCR` 和最少 block 数通过，并记录 detector 仍是主要耗时。
- Chapter 130 已优化翻译同轮去重：前端把同一 OCR 结果里的重复文本只发送一次并回填多个 block，服务端 `BaseTranslator.translate_batch` 对同一请求内重复 miss 去重，`timings` 新增 `request_duplicates`；代码已部署到 N100，LAN smoke 显示 `dup=1`，重复文本只打一次 provider。
- Chapter 131 已把技术文本保护下沉到服务端：路径、命令、文件名、flag、纯中文目标文本等不再进入 provider，`timings` 新增 `preserved_hits`；修复日文汉字+假名被误判为纯中文的 bug，N100 热缓存 smoke 达到 15 blocks / 5 batches 约 `242ms`。
- Chapter 132 已修复截图翻译覆盖渲染的字号和位置：渲染 block 不再合并相邻 OCR 行，译文按原 OCR 框位置逐 block 锁定，字号从原框高度估算且不再为塞入框内压缩到极小，保留显式换行；本地 canvas 视觉回归确认两行译文与原文位置/字号接近。
- Chapter 133 RapidOCR fixture 已覆盖中文大字、英文 UI、小字技术文本、韩文、日文、阿拉伯文；开发版和打包版均通过。常见中英/日文走快速 `ch` 路径约 `1.3–2.1s`，韩文/阿拉伯文触发完整 fallback 约 `5.6–6.3s`，是下一轮性能优化重点。
- Chapter 134 已修复用户真实使用暴露出的交互闭环问题：控制台/识字模型页不再进入即自动跑重诊断；录制保存后回到 ready、保留蓝框并支持第二次录制；视频目录按钮优先打开已保存视频目录；录制控制条黑色阴影已移除；大模型翻译配置支持手填模型、去掉 `(New API)` 叫法，并新增可保存到服务端的翻译 prompt/domain；大模型中转允许用户自托管 LAN/内网地址，N100 已部署。
- `python -m pytest server\tests` 已通过，当前服务端翻译通道完全失败时返回空译文，前端负责拦截和提示恢复路径。
- Snow Shot 风格录制控制条、自动保存、边框状态色、打开目录、复制视频和取消清理已有核心实现与测试基础。

### 6.1.1 当前用户使用路径

- RapidOCR runner 和模型随 Tauri resource 打包，当前主资源目录是 `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner`。
- 用户打开 `.exe` 后，进入“识字模型 / 视频录制”只需要看 RapidOCR 状态、PP-OCRv5/V4 版本选择和自测按钮；普通用户不再选择 OCR 运行时目录。
- 截图 OCR / 翻译使用 `Ctrl+D` 框选文字区域；结果窗必须显示识别文本、截图预览或明确错误，不允许白屏。
- 如果出现 RapidOCR runner 或模型缺失，错误信息必须带 runner/resource 绝对路径，并在配置中心给出刷新、自测和恢复提示。

### 6.2 当前高风险区

| 风险 | 当前真实状态 | 下一步处理 |
|---|---|---|
| RapidOCR 主线仍需真实端到端验收 | 生成式 fixture 和打包 runner fixture 已通过，Rust/前端构建已通过；但仍缺用户真实 `Ctrl+D` 结果窗、真实网页截图、混排长段落和复杂背景验收 | 固化真实截图 fixture，补覆盖层视觉回归，继续记录失败样例并修候选评分/后处理 |
| RapidOCR 打包体积和冷启动 | 清理旧 onefile 遗留后资源约 `340.7 MB`；onedir runner 可用但仍包含 OpenCV/ONNXRuntime/Python 依赖，韩文/阿拉伯文完整 fallback 约 `5.6–6.3s` | 继续裁剪无用 PyInstaller 依赖，按脚本早停，缓存 detector 结果，必要时拆分多语言模型包 |
| 真实 Windows 人工验收不足 | Chapter 110 已验证当前 release 可响应 `Alt+A` 并生成 PNG 全屏捕获；Chapter 126 已有生成式 OCR fixture；Computer Use 无法拖拽透明 overlay，真实 `Ctrl+D` 结果窗仍需要用户或人工验收确认 | 建立人工验收清单和真实截图样例，逐项记录通过/失败/截图证据 |
| 翻译质量仍需样例验收 | Chapter 120 已修复法语/西语被按英文翻译的错误路由，并加入语义关键词 smoke；Chapter 125 已让通道配置和测试状态可见；Chapter 130 已减少重复文本 provider 调用并部署 N100；Chapter 131 已把技术文本保护下沉到服务端并修复日文误伤；但 Google 免费通道仍不是完整商业级多语言质量方案 | 接入更可靠付费/LLM 通道并做真实多语言截图样例验收，继续保留 Google 质量风险提示 |
| 发布商业闭环未完成 | 安装包和 smoke launch 曾通过，但签名、自动更新、模型托管、错误恢复、真实设备矩阵仍未完整 | 建立发布清单、模型源发布流程、真实设备验收和版本回滚流程 |

### 6.3 下一轮优先级

1. Chapter 135：把真实用户截图样例固化进 RapidOCR fixture，重点覆盖清晰英文网页、搜索建议、混排中英、长句和复杂背景。
2. 优化 RapidOCR fallback 性能：先跑 detector 一次，多语言 recognizer 复用检测框，避免韩文/阿拉伯文每个候选重复 detector。
3. 将 RapidOCR candidate summary、selectedLang、耗时和低置信度原因暴露到诊断报告或 OCR 结果调试信息。
4. 继续接入并验证更可靠的付费/LLM 翻译通道，替代 Google 免费通道作为正式质量路线。
5. OCR ready 必须由 RapidOCR 打包 runner、自测、fixture、真实 `Ctrl+D` 结果窗和翻译覆盖层共同证明。

### 6.4 下一执行目标（当前）

目标：在 RapidOCR 主线基础上继续压低多语言 fallback 耗时，并把真实截图失败样例纳入可重复 fixture；录制二次复用和大模型 prompt 配置已补齐，后续继续做真实截图 OCR/翻译质量验收。

必须完成：

- 保持 RapidOCR 生成式 fixture 和打包版 fixture 可复跑。
- 补充真实应用页面、搜索建议、英文长句、技术文本和混排截图样例。
- 优先尝试低风险 detector 复用、脚本早停和候选评分优化，避免牺牲中文和小字召回。

非目标：

- 不承诺完整生产 ready。
- 不恢复旧自研 YSN OCR Runtime 主路径。
- 不恢复 PaddleOCR-json 作为普通主路径。
- 不做发布签名、自动更新、CDN 托管和完整商业发布闭环。

## 7. 当前原则总结

- 商业级优先，不做只为快速落地的短期架构。
- OCR 是战略能力，当前主线是产品内置 RapidOCR / ONNXRuntime runner，而不是旧自研 YSN OCR Runtime。
- 源语言自动识别是硬要求，目标语言默认简体中文并可选。
- 任何 ready 状态必须由真实端到端验证证明。
- 文档只保留主计划和章节日志，代码每章结束必须留下可恢复现场。

