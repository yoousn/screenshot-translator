# 截图唤醒延迟与候选精准度专项计划

> 本文档是 `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` 已索引批准的专项实施计划。它只负责截图唤醒、无闪烁/不卡顿、保存体验、多显示器与候选框精准度的分阶段施工；长期产品方向、OCR 主线和商业级红线仍以主计划为准。

## 0. 当前执行状态：C/E 原生主线接管

- 当前结论：继续微调 WebView 透明壳、1x1 预热或 CSS 遮罩，已经不能稳定消除肉眼闪烁和首帧延迟；该路线暂停作为正式主线。
- 本文档暂不废弃：保留为截图延迟专项的历史基线、验收门禁、Snow Shot 对照线索、候选系统与保存语义索引。
- 新执行主线：先做方案 C「完整原生截图覆盖层」，再做方案 E「DXGI/WGC + GPU texture」，由 `docs/IMPLEMENTATION_CHAPTERS.md` 的后续章节记录实际施工。
- 暂停范围：本文件中关于 WebView 截图页常驻预热、透明 shell、1x1 Draw 窗口、SharedBuffer-WebView 传输的章节暂不继续投入，除非原生主线验证失败需要回退。
- 继续继承：无闪烁、不卡顿、重复热键可取消、Alt-Tab 不污染、保存语义、多显示器候选和 OCR/翻译输出兼容仍然是硬门槛。
- OCR/翻译边界：C/E 主线只替换截图捕获、显示和选区交互层；确认选区后的图片输出必须继续兼容现有 OCR/翻译管线，禁止把 OCR/翻译核心混入本轮改造。

## 1. 当前目标

- 第一优先级：`Alt+A` 在按下 `A` 的瞬间尽可能触发截图态，体感接近 Snow Shot 的 `Ctrl+Alt+A`，避免 1–2 秒等待。
- 第二优先级：截图态不能闪屏、不能明显卡顿，鼠标移动和候选框响应要顺滑。
- 第三优先级：`Ctrl+S` 和工具栏保存必须响应明确；普通保存走原生 Save dialog，默认真实系统桌面；保存成功后退出截图态。
- 第四优先级：候选框先覆盖显示器/窗口，再逐步覆盖 UIAutomation 控件和视觉候选；鼠标在桌面时也能框选到当前显示器。
- 第五优先级：每一步都小步验证；用户构建运行确认无回归后，再进入下一步。

## 2. 硬性约束

- 不恢复会造成闪烁的“先显示空覆盖层、再等截图”的激进方案，除非后续能做到真正隐藏预热且没有肉眼闪烁。
- 不用 opaque 外部截图进程替代产品主路径；可以参考 Snow Shot，但实现必须可维护、可拥有。
- Snow Shot 公开项目存在 GPL/Commercial 双许可线索；本项目只借鉴产品架构和性能模式，不复制实现代码或资源。
- 保存行为必须分清 `Save As` 与未来可选 `Fast Save`：当前普通保存应弹出原生保存窗口。
- 默认保存目录必须使用系统真实 Desktop API，不能硬编码 `C:\Users\...\Desktop`，以兼容桌面被迁移到 D/E/F 盘。
- 后续每章只解决一个主要瓶颈，不同时大改热键、捕获、候选、保存和 OCR。
- 每章必须保留 `cargo check`、`npm run build`、真实手动 smoke 和延迟日志记录。

## 3. 当前已知状态

| 项目 | 当前状态 | 风险 |
| --- | --- | --- |
| `Alt+A` 唤醒 | 已从 base64 大事件改成文件 payload，之前测到前端图片 ready 约 `0.76–0.91s` | 覆盖层仍要等截图图像 ready 才显示，体感不够瞬间 |
| 闪烁 | 已撤回预捕获可见覆盖层，当前“不闪”优先 | 如果再次提前 show 窗口，必须先做隐藏预热和 DWM/动画处理 |
| `Ctrl+S` | 已改为原生 Save dialog，默认系统 Desktop | 保存成功后仍需确认自动退出截图态 |
| 候选框 | 当前以 Win32 window rect + 可选视觉候选为主 | 缺显示器候选、UIAutomation/RTree、虚拟桌面坐标统一 |
| 多显示器 | 当前以当前显示器截图为主 | 鼠标在桌面时不框显示器；跨屏与 DPI 风险未闭合 |
| Snow Shot 线索 | 源码保留在 `C:\Users\ysn\AppData\Local\Temp\snow-shot-src` | 不能删除，后续继续参考 |

## 4. 外部与竞品参考结论

- Tauri 官方 `WebviewWindow` / `Window` API 支持窗口可见性、show/hide 和事件监听；方向上应避免每次热键重新创建 WebView，而是复用已加载的截图页。
- Microsoft Desktop Duplication API 通过 DXGI surface 获取桌面帧，支持 dirty/move rect 和多显示器旋转处理；后续如果当前截图库成为瓶颈，应评估 GPU/原始 BGRA 捕获主线。
- Microsoft `SetWindowDisplayAffinity` 的 `WDA_EXCLUDEFROMCAPTURE` 可让自己的覆盖窗口不进入捕获画面，但只适用于当前进程顶层窗口，并且 Windows 10 2004 前兼容行为不同；继续用它可以减少“先显示 overlay 再捕获”的自我捕获风险。
- Microsoft UIAutomation `BoundingRectangle` 可提供控件边界；Snow Shot 结合 UIA、Win32、RTree 和前端索引来提升候选框精准度，这比单纯 Win32 窗口矩形更接近专业截图工具。
- Snow Shot 关键模式：预创建隐藏 1x1 draw 窗口、禁用窗口过渡、并行捕获/窗口准备、全屏虚拟桌面捕获、共享缓冲区传输、UIA/RTree 候选、多显示器配置、独立 fast save 设置。

参考链接：

- Tauri WebviewWindow API：https://v2.tauri.app/reference/javascript/api/namespacewebviewwindow/
- Microsoft Desktop Duplication API：https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
- Microsoft SetWindowDisplayAffinity：https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity
- Microsoft UIAutomation BoundingRectangle：https://learn.microsoft.com/en-us/dotnet/api/system.windows.automation.automationelement.automationelementinformation.boundingrectangle

## 5. Snow Shot 可借鉴清单

| 能力 | Snow Shot 线索 | 本项目计划映射 |
| --- | --- | --- |
| 预热截图窗口 | 预创建隐藏 1x1 Draw 窗口 | Chapter A：截图页常驻预热与无闪窗口生命周期 |
| 热键发事件给已加载页面 | `execute-screenshot` 事件进入 Draw 页面 | Chapter A：从“构建会话后显示”改为“已加载页面接事件” |
| 并行唤醒状态机 | 捕获所有显示器和显示窗口准备并行 | Chapter A/C：减少串行等待 |
| 多显示器捕获 | `capture_all_monitors`、`enableMultipleMonitor` | Chapter D：虚拟桌面画布与显示器候选 |
| 原始/共享缓冲区 | `create_webview_shared_buffer`、`WebViewSharedBufferState` | Chapter C：替代 PNG/base64/file decode 热路径 |
| UIA 控件候选 | `findChildrenElements` 与 UIA walker | Chapter E：UIA/RTree 候选层 |
| 候选索引 | 前端 Flatbush/RTree 查询 | Chapter E：前端空间索引 |
| 保存体验 | `fastSave` 与普通保存分离 | Chapter B/G：保存后退出；后续可加 Fast Save 设置 |
| 保存/复制并行 | 后端 save/copy 可并行执行 | Chapter B/G：未来可做组合动作，但普通 `Ctrl+S` 仍先 Save As |
| 截图历史 | 独立 history image pool | Chapter G/H：后续支持误关恢复和 OCR 复用，不进首轮热路径 |
| 动画控制 | `disableAnimation`、DWM transitions | Chapter A/F：禁动画、发布模式确认 |
| HDR/颜色 | `correctHdrColor` | Chapter H：颜色/HDR 质量优化 |
| 专业 UI | 左侧快捷键提示、状态栏候选模式 | Chapter G：交互 polish，不抢先做 |
| 热加载页面池 | idle WebView/page pool，释放中可重入唤醒 | Chapter A：连续截图时减少关闭/重建抖动 |
| 隐藏态鼠标穿透 | hidden/opacity 0/ignore cursor events 状态机 | Chapter A：预热时不挡鼠标、不闪烁 |
| 色彩滤镜校正 | `correctColorFilter` | Chapter H：不只 HDR，也覆盖 Windows 无障碍色彩滤镜 |
| 全屏误触保护 | `disableOnFocusedFullScreenWindow` | Chapter H：游戏/演示/全屏视频中降低误触发 |

## 6. 分阶段执行计划

### Chapter 0：基线打点与真实瓶颈确认

目标：先不改变行为，只补齐可量化日志和手动测试表，确认真正瓶颈在窗口生命周期、捕获、PNG 编码、磁盘写入、IPC、前端 decode、canvas paint、候选加载还是保存写入。专业判断：没有基线就直接上 SharedBuffer、DXGI/WGC 或全量 UIA，容易投入很大但收益不确定。

可能触及文件：

- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotActions.ts`
- `tauri-client/src/hooks/useScreenshotWindowRects.ts`

实施细节：

1. 为每次截图生成 `sessionId`，贯穿 Rust 和前端日志。
2. Rust 侧记录：`hotkey_received`、`debounce_passed/rejected`、`main_hidden`、`capture_start`、`capture_end`、`png_encode_end`、`backup_write_start/end`、`payload_emit`、`overlay_show_called`、`overlay_show_result`。
3. 前端记录：`payload_received`、`file_load_start/end`、`image_decode_start/end`、`mask_canvas_ready`、`analysis_image_data_ready`、`first_paint`、`overlay_ready_to_show_called`、`candidate_first_batch`。
4. 保存记录：`save_invoked`、`dialog_open_start/end`、`dialog_cancelled`、`output_cache_hit/miss`、`output_render_end`、`file_write_start/end`、`overlay_exit_after_save`。
5. 日志尽量结构化为单行 JSONL 或稳定 key-value，避免自由文本难以统计，也避免中文编码影响延迟分析。
6. dev 和 release 各测 10 次冷/热 `Alt+A`；如果 release 明显更快，后续以 release 作为体感验收主依据。

验收门禁：

- 至少能从日志自动或手动算出 `hotkey->visible`、`hotkey->image-ready`、`capture`、`decode`、`save-dialog`、`save-write`。
- 日志中不能出现空 payload、`NaN` 坐标、重复 session、丢失 session。
- Chapter 0 不改变截图行为；如果行为变化，视为失败。

暂停点：用户看过基线数据后，再决定 Chapter A 优先修哪一段。

### Chapter A：无闪常驻截图页与热键入口

目标：让截图 WebView 在应用启动后提前加载并保持隐藏/1x1/不可见，`Alt+A` 只做状态切换和捕获启动，减少 WebView 创建、页面加载、React 初始化和资源加载带来的延迟。

可能触及文件：

- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/index.css`

实施细节：

1. 审计当前截图窗口创建、定位、show、focus、capture exclude 顺序。
2. 将截图窗口生命周期拆成 `ensure_screenshot_window_preloaded`、`prepare_screenshot_session`、`show_ready_overlay`、`hide_screenshot_overlay` 四个边界。
3. 应用启动时创建隐藏窗口，初始尺寸 1x1 或移出可见区域，禁任务栏、禁阴影/动画、设置 capture exclude。
4. 前端截图页 mounted 后上报 `screenshot-page-ready`，Rust 侧记录 ready 状态；热键发生时 ready 才直接发事件，未 ready 走 fallback 并记录冷启动原因。
5. `Alt+A` 按下后先记录 `hotkey_received_at`，再并行开始捕获和候选预取；只有图像或安全占位真正 ready 时才显示，避免闪。
6. 逐步测试 `hide -> resize/reposition -> emit -> show/focus` 与 `resize/reposition -> show -> emit` 哪种更平滑，只保留无闪版本。
7. 把临时 PNG 备份写盘移出窗口出现前关键路径；如果仍需 debug 备份，只能后台写，不允许 `hotkey->visible` 等待磁盘 IO。
8. 前端首屏显示不等待 UIA、视觉分析、OCR 预热或重组件加载；这些全部异步进入。
9. 增加热键去抖与重入日志：记录被 450ms 去抖、`CAPTURING` 重入、`Alt+A/Alt+T/Alt+R` 串台拦截的具体原因。
10. 增加截图页 ready 门禁：只有前端 mounted、事件 listener 已注册、canvas 容器可用后，Rust 才把它视为可接收截图 payload。
11. 增加 overlay show 确认回传：`hotkey->visible` 不只记录调用 show，而要记录窗口实际 visible/focused 或至少 show/focus 成功结果。
12. 增加隐藏态鼠标穿透状态机：预热隐藏时不挡鼠标，进入截图态再恢复输入；退出后延迟释放，释放中再次热键要能重入唤醒。
13. 主窗口隐藏/恢复要覆盖三种 smoke：主窗口原本可见、最小化、隐藏，避免截图后幽灵弹出或焦点丢失。

验收门禁：

- `Alt+A` 第一次冷启动窗口路径有日志：`hotkey -> window-ready -> capture-start -> image-ready -> visible`。
- 热启动连续 10 次，肉眼无白闪、黑闪、透明层闪、窗口跳动。
- 鼠标移动不掉帧，截图页 ready 后不会阻塞主窗口。
- 如果用户观察到闪烁，本章立即回滚，不进入下一章。
- 连续快速 `Alt+A -> Esc -> Alt+A` 不应因为窗口释放中、热键去抖或 listener 未 ready 导致空等或失败。
- 主窗口原本可见、最小化、隐藏三种状态下，截图退出后的恢复行为符合预期。

验证命令：

- `cd tauri-client/src-tauri; cargo check`
- `cd tauri-client; npm run build`
- `git diff --check`

用户手动 smoke：

- 构建运行后，连续按 `Alt+A` 10 次，每次按 `Esc` 退出。
- 记录每次从按下 `A` 到截图态出现的体感：`瞬间 / <300ms / 300-700ms / >700ms`。
- 观察是否闪烁、是否抢焦点失败、是否出现空白截图页。

暂停点：用户确认“无闪且更快”后才进入 Chapter B。

### Chapter B：保存后退出与 Save As 状态机

目标：明确普通保存语义：`Ctrl+S` / 工具栏保存先弹出原生 Save dialog；Save dialog 打开期间必须保持当前截图上下文；用户确认保存并写入成功后顺便退出截图状态；用户取消则继续停留当前截图状态，选区、标注、翻译覆盖和工具栏状态都不丢失。

可能触及文件：

- `tauri-client/src/hooks/useScreenshotActions.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`

实施细节：

补充产品语义：

- 普通保存必须采用类似 Snow Shot 的 `Save` 语义，而不是 `Fast Save` 语义。
- `Ctrl+S` 与工具栏保存按钮都触发普通保存；不得一个走普通保存、另一个走快存或旧的 base64 直存命令。
- 普通保存第一步必须弹出原生 Save dialog，并默认定位到系统真实 Desktop。
- Save dialog 打开期间，当前截图上下文必须保持有效：选区、截图背景、标注、翻译结果、工具栏状态和 warmed output/cache 不应被清空。
- 用户在 Save dialog 中取消时，不退出截图状态，不隐藏 overlay，不清空选区，不显示“操作失败”类错误；用户仍可继续复制、重新保存、调整选区或按 Esc 取消。
- 用户选择路径后，才生成/读取当前输出并写入文件；保存后的退出必须发生在写入成功之后，而不是用户刚选中路径之后。
- 写入失败时不退出截图状态，必须显示可恢复错误，并允许用户再次保存或取消。
- Save dialog 是应用外模态交互，期间 WebView 可能失焦；不能依赖 focus/blur 或窗口隐藏事件清理截图状态。
- Save dialog 打开时推荐冻结截图内部编辑交互但保留视觉上下文；dialog 关闭后恢复，避免背景状态静默变化导致保存内容与用户确认前不一致。
- `emit("screenshot-captured", base64)` 是否在保存成功后触发需要保持现有语义；保存失败或取消时不得发送“已完成截图”类事件。

1. 保持 `choose_image_save_path` 默认 `dirs::desktop_dir()`，不硬编码桌面路径。
2. 明确 `save` 成功后的状态机：写入成功 -> 清理 warmed output/cache -> hide overlay -> 恢复焦点策略。
3. 用户取消 Save dialog 时不退出截图态，让用户可以继续复制、重新框选或按 Esc。
4. 保存失败时不退出截图态，显示可恢复错误；错误不再塞在截图窗口内部 message。
5. 避免在弹出 Save dialog 前生成大 PNG 阻塞 dialog；先让用户选路径，再用 warmed output 或后台输出写入。
6. 保存动作增加“防重复触发”状态：Save dialog 或写盘过程中再次按 `Ctrl+S` 不重复弹窗、不重复写入。
7. 未来再加独立 `Fast Save`：配置目录、文件名模板、保存后 toast、打开文件夹；不覆盖普通 `Ctrl+S` 的 Save As 语义。
8. Save dialog 打开时保持截图态：不能提前清空 selection、不能提前隐藏 overlay、不能释放截图窗口、不能丢失 warmed output。
9. 保存成功退出截图态必须发生在 `write_image_to_file` 成功之后；写入失败时保持截图态并允许用户重试保存。
10. 增加 `isSavingRef` / toolbar busy / 快捷键 busy 门禁：双击保存、连续 `Ctrl+S`、dialog 打开时再次 `Ctrl+S` 都不能重复弹窗或重复写入。
11. 输出缓存版本化：用显式 `selectionVersion`、`annotationVersion`、`translationVersion` 判断缓存，而不是只看 JSON 长度或对象顺序。
12. 保存失败提示后续应迁移到桌面级通知或主窗口提示；当前章节先保证失败不退出、可重试。

验收门禁：

- `Ctrl+S` 弹出原生 Save dialog，默认目录是真实系统 Desktop。
- 选择保存后截图窗口退出；文件存在且能打开。
- 点击工具栏保存与 `Ctrl+S` 行为一致。
- 取消保存不退出截图态，当前选区、标注、翻译结果和工具栏状态保持不变。
- 保存成功后退出截图态，且只在文件实际存在、大小非零、可打开后记录成功。
- 保存失败不退出截图态，可再次按 `Ctrl+S` 重试。
- Save dialog 打开期间，截图窗口不被销毁，截图上下文不丢失。
- `Ctrl+S` 弹出原生 Save dialog，默认目录为 `dirs::desktop_dir()` 返回的真实系统桌面；桌面迁移到 D/E/F 盘时不能回落到硬编码 `C:\Users\...\Desktop`。
- 在 Save dialog 中点击取消：截图态继续停留，选区和工具栏仍可操作，不出现失败 toast。
- 在 Save dialog 中选择路径并保存成功：目标 PNG 存在、大小非零、可打开，随后 overlay 退出并恢复主窗口/焦点策略。
- 写入失败或目标路径不可写：overlay 不退出，显示明确错误和下一步，例如“请重新选择保存位置或按 Esc 取消”。
- 保存过程中连续按 `Ctrl+S` 5 次或连续点击工具栏保存：最多出现一个 Save dialog，最多写入一次，状态机不乱序。
- `copy`、`pin`、`OCR/translate` 等非保存动作不受保存状态锁影响，除非当前正在写盘且会破坏同一输出缓存。
- 多显示器/DPI 后续章节改变选区坐标来源后，主屏、副屏和负坐标屏都要重新 smoke 保存输出，避免保存图像与屏幕选区偏移。


验证命令：

- `cd tauri-client/src-tauri; cargo check`
- `cd tauri-client; npm run build`
- `git diff --check`

用户手动 smoke：

- 框选区域后按 `Ctrl+S`，保存到桌面，确认 overlay 退出。
- 再次截图，点击工具栏保存，确认 overlay 退出。
- 第三次截图，按 `Ctrl+S` 后取消，确认仍停留截图态。

暂停点：用户确认保存体验正确后才进入 Chapter C。

### Chapter C：原始缓冲区/共享缓冲区传输

目标：把截图图像从 PNG 文件/解码热路径逐步迁移到原始 BGRA/RGBA buffer 或 WebView shared buffer，减少 PNG 编码、文件 IO、前端 decode 带来的 `0.7–0.9s` 图像 ready 延迟。

可能触及文件：

- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- 新增 `tauri-client/src-tauri/src/screenshot_buffer.rs`
- 新增 `tauri-client/src/utils/screenshotBuffer.ts`

实施细节：

1. 先加打点，不改行为：拆出 capture、PNG encode、file write、IPC emit、frontend fetch、image decode、canvas paint。
2. 设计 buffer payload：`width`、`height`、`stride`、`format`、`scaleFactor`、`monitorOrigin`、`bufferId`。
3. 第一版使用内存缓存 + 小 IPC metadata，前端再通过命令取 `Uint8Array`；如果 Tauri IPC 对二进制不稳定，再评估 shared buffer/channel。
4. 前端使用 `ImageData` / `createImageBitmap` / canvas 绘制，避免 data URL 和大 base64。
5. 保留 PNG fallback：buffer path 失败时回到当前文件 payload，不能让截图不可用。
6. 对比 Snow Shot shared buffer，但不直接复制不可维护结构。
7. SharedBuffer 必须有能力检测、WebView2 版本门控、失败 fallback、buffer release 生命周期和内存泄漏验收；不能只追求快而让大图 buffer 常驻泄漏。
8. file payload 路径必须有竞态门禁：文件存在、大小稳定、加载失败自动 fallback；如果备份写盘还在关键路径，必须先移出关键路径再评估 buffer 收益。
9. 同一选区的前端 crop、Rust crop、保存输出、OCR 输入像素必须一致，作为物理/逻辑坐标验收。

验收门禁：

- 图像 ready 热启动目标：优先压到 `<300ms`；如果当前截图库本身慢，至少明确瓶颈在 capture 而不是传输/解码。
- 画面颜色、缩放、DPI、选区坐标不偏移。
- 保存、复制、OCR、翻译仍能从同一源图生成 PNG。

验证命令：

- `cd tauri-client/src-tauri; cargo check`
- `cd tauri-client; npm run build`
- `git diff --check`

用户手动 smoke：

- 连续 10 次 `Alt+A` 记录 `image-ready` 日志。
- 选区复制、保存、OCR 各做一次，确认输出图像一致。

暂停点：用户确认延迟明显下降且无图像偏移后进入 Chapter D。

### Chapter D：多显示器虚拟桌面画布与显示器候选

目标：鼠标在桌面空白处时也能框到当前显示器；跨多屏时坐标、DPI、截图画布和候选框统一到虚拟桌面坐标。

实施细节：

1. 增加 `get_monitors_snapshot`：返回每个 monitor 的 origin、size、scale、is_primary、name、work_area。
2. 候选列表最底层永远加入 monitor rectangles，优先级低于窗口/控件但高于视觉空候选。
3. 若当前捕获仍只支持单显示器，先在当前显示器内做候选；虚拟桌面全量捕获作为后续小步。
4. 逐步支持 `capture_all_monitors`：将所有显示器拼成 virtual desktop canvas，记录每屏偏移和 scale。
5. 前端所有鼠标坐标、选区坐标、候选坐标统一转换，避免负坐标/副屏偏移。
6. 区分 full monitor 与 work area：任务栏、自动隐藏任务栏、顶部 Dock 类工具都要测试候选是否偏移或缺失。
7. 增加空间索引双向转换验收：窗口坐标 ↔ 显示器坐标 ↔ virtual desktop 坐标 ↔ RTree 命中坐标必须可追踪。
8. 全量虚拟桌面捕获不和显示器候选强绑；先做显示器候选和坐标统一，数据证明需要后再做 `capture_all_monitors`。

验收门禁：

- 鼠标放桌面空白处，显示器候选框出现且大小正确。
- 主屏、副屏、负坐标屏都不偏。
- 单屏用户没有行为变化。

暂停点：显示器候选稳定后进入 Chapter E。

### Chapter E：Win32 + UIAutomation + RTree 多层候选

目标：把候选框从“窗口矩形为主”升级为“显示器 + Win32 窗口 + UIA 控件 + 视觉候选”的多层系统，提升 UI 窗口识别专业度和精准度。

实施细节：

1. 保留现有 Win32 window rect，不破坏当前行为。
2. 新增 UIA rect command，限制扫描范围、超时和最大节点数，避免 UIA 卡住热路径。
3. UIA 候选异步加载：截图态先可用 monitor/window 候选，UIA 候选随后增量注入。
4. 前端建立 RTree/Flatbush 或轻量空间索引，用鼠标点快速查找最合适候选。
5. 候选评分规则：点命中 > 面积适中 > 层级更深 > 前台进程 > 可见性 > 非工具窗口。
6. 提供候选循环能力：同一点多个候选时可通过快捷键或后续 UI 循环选择。
7. UIA 不做全桌面全树扫描；默认只查鼠标附近、前台窗口或命中窗口子树，并按鼠标命中点逐步展开子节点/兄弟节点。
8. 候选后端必须有超时、最大候选数、取消过期请求、增量版本号；鼠标移动时旧请求返回不能覆盖新候选。
9. 候选排序模型必须显式定义：窗口序、元素层级、面积、父子关系、前台进程、可见性、控件类型共同决定默认框。

验收门禁：

- UIA 失败不会影响截图出现速度。
- 常见应用窗口、按钮区域、浏览器内容区候选比当前更准。
- 桌面空白仍回退到显示器候选。

暂停点：候选精准度改善且不卡后进入 Chapter F。

### Chapter F：Release 构建与运行时性能档位

目标：确认用户实际使用的版本是 release 优化路径，避免 dev 模式、debug assertions、未压缩前端 chunk 或开发日志影响体感。

实施细节：

1. 对比 `npm run tauri dev` 与 release 便携包的 `Alt+A` 延迟。
2. 检查 Rust profile、LTO、codegen-units、panic strategy 是否适合发布。
3. 检查前端 chunk、source map、开发日志和 Ant Design 体积警告是否影响截图页首次 ready。
4. 保留诊断日志开关，发布默认只记录关键性能点。
5. 不为追求速度删除错误恢复、OCR 主线和保存 fallback。
6. 将 release profile、LTO、strip、codegen-units、debug assertions、source map、前端 chunk 体积纳入延迟验收。
7. 增加最小截图专项 smoke：固定日志字段，校验无 `NaN`、无空 payload、保存文件存在、保存后状态正确。
8. 增加 UTF-8/结构化 JSONL 日志门禁，避免中文日志或计划文档乱码影响后续分析。

暂停点：确认 release 表现后再做 polish。

### Chapter G：截图态交互 polish

目标：在速度和候选稳定后，补专业截图工具交互，不提前影响性能主线。

候选优化：

- 左侧快捷键提示：复制、保存、OCR、翻译、退出、候选循环。
- 状态栏显示当前候选来源：显示器 / 窗口 / 控件 / 视觉。
- 候选循环：同点多个候选时切换父/子层级。
- 选择动画可配置关闭，默认保证低延迟。
- `Fast Save` 作为单独设置，不覆盖普通 Save As。
- 截图历史作为独立资产池，支持误关恢复、重新保存和 OCR/翻译复用，但不得拖慢 `Alt+A` 首屏。
- 保存与复制组合动作并行化：未来 `save+copy` 或 OCR 复用可以并行执行，减少用户等待。

### Chapter H：颜色、HDR 与边缘场景

目标：解决高 DPI、HDR、缩放、远程桌面、管理员权限应用等边缘问题。

候选优化：

- HDR 色彩校正开关，参考 Snow Shot `correctHdrColor`。
- DPI mixed-mode 坐标审计。
- 管理员权限窗口 UIA 失败提示与 fallback。
- RDP/虚拟显示器 capture fallback。
- 受保护窗口/黑屏捕获的错误提示。

### Chapter I：关于页产品信息收口

目标：把关于页从偏技术说明的长内容收敛成简单、优雅、可信的产品信息面板，展示版本号、作者、仓库和联系方式。当前关于页旧内容后续实现时清理掉，不在旧内容上追加卡片；该章节只属于低风险 UI 收口，不影响截图热路径。

产品信息要求：

1. 展示当前应用版本号，优先读取 `package.json` / Tauri 构建元数据，不能继续硬编码 `v1.0.0`。
2. 展示作者：`犹少年`。
3. 提供 GitHub 仓库按钮，链接到 `https://github.com/yoousn/screenshot-translator`。
4. 展示联系方式：`gg1761229856@gmail.com`。
5. 页面风格简单、紧凑、现代：一个主信息卡 + 少量操作按钮即可，不做大段技术营销文案。
6. 技术栈鸣谢可以保留为次级折叠/轻量区域，但不应压过产品身份、版本、作者和联系方式。
7. 外链打开必须走安全外部打开方式，不在 WebView 内跳转破坏应用状态。
8. 文案避免“Licensed under MIT”等未在发布策略中确认的信息，除非主计划明确确认许可证。

验收门禁：

- 关于页可见应用名称、版本号、作者“犹少年”、GitHub 仓库按钮和邮箱。
- 版本号与当前构建版本一致；当前 `tauri-client/package.json` 为 `1.2.5` 时，关于页不应显示 `v1.0.0`。
- 点击 GitHub 仓库按钮会打开 `https://github.com/yoousn/screenshot-translator`。
- 邮箱可复制或可通过 `mailto:` 打开，至少一种路径明确可用。
- 页面在 800px 宽度内不拥挤，不出现大段无关技术介绍。
- 不引入新的长期计划文档；本小章记录在本文档，完成后只在 `docs/IMPLEMENTATION_CHAPTERS.md` 写章节历史。

风险：

- 版本号如果从前端常量读取，容易与 Tauri 打包版本、安装包版本不一致；应建立单一来源或构建期注入。
- GitHub 外链如果直接用普通 `<a target="_blank">`，可能在 WebView 内打开或被 CSP/权限影响；应确认 Tauri opener 权限和行为。
- 关于页当前内容偏“技术栈介绍”时，不能直接叠加更多卡片；应重构为产品身份优先。
- 许可证、商业发布、作者署名属于产品对外信息，不能写未经确认的声明。

## 7. 延迟测试记录模板

每章都追加真实测试结果，不用主观描述替代数据。

| 日期 | 章节 | 构建模式 | 测试次数 | hotkey->visible | hotkey->image-ready | save dialog open | save write | 是否闪烁 | 是否退出 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 待填 | A | dev/release | 10 | 待填 | 待填 | - | - | 待填 | Esc | 待填 |
| 待填 | B | dev/release | 5 | 待填 | 待填 | 待填 | 待填 | 待填 | 待填 | 待填 |

目标分档：

- 优秀：`hotkey->visible <100ms`，`hotkey->image-ready <300ms`，无闪烁。
- 可接受：`hotkey->visible <200ms`，`hotkey->image-ready <600ms`，无闪烁。
- 需继续优化：`hotkey->visible >300ms` 或有任何闪烁/卡顿。
- 不通过：保存无响应、保存后不退出、候选偏移、截图黑屏、OCR/复制/翻译回归。

## 8. 清理计划

已清理并保留在 git diff 中：

- `.codebuddy/settings.json`
- `GHOST_WINDOW_FIX_HANDOFF.md`
- `测试图片/` 下旧图片 fixture
- 根目录生成物 `YsnTrans.exe` / `YsnTrans.pdb` 如存在已移除

明确保留：

- `C:\Users\ysn\AppData\Local\Temp\snow-shot-src`：Snow Shot 源码线索，后续继续参考，不删除。
- `C:\Users\ysn\Desktop\Snow Shot_0.7.8-beta_windows-x64-portable`：竞品行为对照，不删除。

暂不删除，需用户确认：

- `check_commercial.ps1`：商业检查入口，建议保留。
- `deploy_n100_translation_server.ps1`：翻译服务部署脚本，建议保留，除非 N100 路线废弃。
- `pack_release.ps1`：发布打包入口，必须保留。
- `refresh_windows_icon_cache.ps1`：图标刷新辅助，建议保留。
- `scripts/check-icon-contrast.ps1`：图标质量检查，建议保留。
- `scripts/sync-app-icons.ps1`：图标同步，建议保留。
- `tauri-client/scripts/build-rapidocr-runner.ps1`：OCR runner 构建，必须保留。
- `tauri-client/scripts/check-ocr-fixtures.ps1`：OCR fixture 门禁，必须保留。
- `tauri-client/node_modules/.bin/*.ps1`：依赖生成脚本，不手动删除；如要清理应删除整个 `node_modules` 后重新安装。

后续清理规则：

- 只删除确认无用的临时计划、生成物、测试截图和工具缓存。
- 不删除 release、OCR、部署、图标、fixture、Snow Shot 参考相关脚本。
- 每次清理先列清单，再执行删除，并在 `docs/IMPLEMENTATION_CHAPTERS.md` 记录。

## 9. 专业审稿后的优先级调整

最终优先级不完全按最初章节顺序执行，后续用 `$grill-me` 挑选时建议优先质询这些取舍：

1. **Chapter 0：基线打点与 release/dev 对比**。先确认真实瓶颈，避免盲目上 SharedBuffer、DXGI/WGC 或全量 UIA。
2. **Chapter A1：热路径低风险修复**。移出备份写盘、确认截图页 ready、确认 overlay show 结果、确保 OCR/UIA/视觉分析不阻塞首屏。
3. **Chapter A2：无闪显示时序**。只比较小范围 show/hide/focus 顺序，不重新引入会闪的空覆盖层。
4. **Chapter B-lite：保存状态机补齐验证**。重点是 Save dialog 期间保持截图上下文、确认保存成功后退出、取消继续、失败可重试、防重复。
5. **Chapter D1：显示器候选与坐标统一**。先解决桌面空白处框显示器和 full monitor/work area 坐标问题。
6. **Chapter C1：传输/解码优化试验**。基于 Chapter 0 数据决定是否做内存 buffer、SharedBuffer、`ImageData/createImageBitmap`。
7. **Chapter E：UIA/RTree 异步候选**。只做异步、限流、可降级的控件级候选。
8. **Chapter D2/C2：全多屏捕获或捕获 backend AB**。只有数据证明当前捕获库是瓶颈，才评估 DXGI Desktop Duplication / Windows Graphics Capture。
9. **Chapter G/H：polish、HDR、色彩滤镜、RDP、管理员窗口、全屏误触保护**。稳定后作为商业级边缘质量推进。

## 10. 关于面板与产品信息计划

目标：在后续 UI polish 阶段，为应用增加一个简单、优雅、不打扰主流程的“关于”信息入口。当前只写入计划，不改代码。

建议内容：

- 应用名称：截图翻译 / Screenshot Translator。
- 版本号：展示当前应用版本，来源优先使用 Tauri/package 版本，避免手写错版本。
- 作者：犹少年。
- GitHub 仓库：`yoousn/screenshot-translator`，显示为按钮或超链接按钮，地址为 `https://github.com/yoousn/screenshot-translator`。
- 联系方式：`gg1761229856@gmail.com`。

设计要求：

- 入口可以放在设置页、主面板页脚或标题栏菜单；不要打断截图热路径。
- GitHub 链接必须是可点击按钮，并通过系统浏览器打开。
- 关于面板保持轻量，不塞大段营销文案；风格要和商业级紧凑 UI 一致。
- 版本号、作者、联系方式允许复制，方便用户反馈问题。

建议归属：Chapter I：关于页产品信息收口，不进入 Chapter A/B 热路径。

## 11. 后续可追加优化

- 进程启动后后台预热截图依赖和候选扫描依赖，但不能占用明显 CPU。
- 热键低级路径优化：减少配置读取、锁等待、窗口查找和重复注册。
- 截图页资源瘦身：把 OCR/翻译重组件从普通截图态首屏拆懒加载。
- 候选预取：鼠标位置、前台窗口和显示器快照可在热键前低频缓存。
- 捕获 backend AB 测试：当前库、DXGI Desktop Duplication、Windows Graphics Capture 分别测冷/热延迟。
- 独立桌面通知：保存成功后退出截图态，再用系统通知或主窗口轻提示展示结果。
- 性能诊断面板：用户可导出最近 20 次 `Alt+A` 和 `Ctrl+S` 延迟。

## 12. 下一步执行顺序

1. 先由用户用 `$grill-me` 或人工审阅挑选本计划中的优化项。
2. 用户确认后，优先只做 Chapter 0，不碰功能行为。
3. Chapter 0 完成后，用户构建运行并提供 dev/release、冷/热启动延迟基线。
4. 基线确认后再做 Chapter A1；A1 无闪、不卡、明显更快后，再做 Chapter B-lite。
5. 每章结束更新 `docs/IMPLEMENTATION_CHAPTERS.md`，记录真实延迟、失败样例和下一章建议。

## 13. Grill-Me 已确认设计决策

本节记录用户通过 `$grill-me` 已确认的第一轮设计边界。后续实现必须以本节为准；未列入第一轮的能力只能作为后续章节，不得混入当前实现。

### 13.1 截图唤醒底线

- 已确认：稳定优先，`Alt+A` 唤醒必须无闪烁、不卡顿、可重复。
- 更快但会闪、会卡、会出现空 overlay 的方案，只允许作为实验对比，不允许进入正式版本。
- Snow Shot 参考路线是预建隐藏截图窗口、禁动画/阴影/DWM 过渡、热键发事件给已加载页面、捕获和窗口准备并行、大图走 SharedBuffer；本项目第一轮只借鉴低风险部分，不直接照搬大图通道和完整候选系统。

### 13.2 第一轮范围

已确认第一轮只覆盖：

1. `Chapter 0`：基线打点与真实瓶颈确认。
2. `A1`：低风险唤醒热路径修复。
3. `A2`：无闪显示时序打磨。
4. `B-lite`：普通 Save As 保存语义补齐。
5. `D1`：桌面空白处完整显示器候选、任务栏整体候选、坐标统一。
6. `Chapter I`：关于页产品信息收口。

第一轮明确不做：

- `SharedBuffer` 正式接入。
- DXGI Desktop Duplication / Windows Graphics Capture 捕获后端替换。
- 全量 UIAutomation / RTree 控件候选。
- Fast Save。
- HDR / 色彩滤镜 / RDP / 管理员窗口等边缘增强。

### 13.3 验收方式与目标阈值

- 已确认采用“体感 + 数据双轨”。
- 体感硬门槛：无闪、不卡、顺滑、可重复。
- 数据必须记录：`Alt+A -> overlay 可交互`、`Alt+A -> image ready`。
- 第一轮目标很激进：release 热启动 `Alt+A -> 可交互 <150ms`，`Alt+A -> image ready <300ms`。
- 该目标是优化方向，不是 Chapter 0/A1 的失败门槛；如果基线证明当前捕获/解码天然超过目标，先记录差距，再决定后续是否进入 `SharedBuffer` 或捕获后端路线。

### 13.4 普通保存语义

- 已确认：第一轮不做 Fast Save。
- `Ctrl+S` 和工具栏保存完全一致，都是普通 Save As。
- 保存第一步弹原生 Save dialog，默认真实系统 Desktop。
- Save dialog 打开期间保持截图上下文，不清空选区、标注、翻译结果、工具栏状态或 warmed output/cache。
- 用户取消保存：继续停留当前截图态，不报错。
- 用户确认并写入成功：退出截图态，并显示轻量反馈 `已保存到 ...`，最好带“打开文件夹”。
- 写入失败：不退出截图态，显示明确错误和重试路径。
- 成功反馈不得显示在截图窗口内部；应使用退出后的桌面级 toast、系统通知或主窗口轻提示。

### 13.5 候选系统第一轮范围

- 桌面空白处候选已确认使用完整显示器，不使用 work area 作为第一优先。
- 第一轮候选优先级：窗口 / 任务栏整体 > 显示器 > 视觉候选。
- 任务栏要作为类似独立窗口 UI 的整体候选纳入 D1；如果实现困难，至少把任务栏候选写成明确后续 D1.5，不阻塞第一轮核心目标。
- 任务栏内部细分暂不做：开始按钮、任务按钮区、右侧托盘区以后再通过 UIAutomation / 子窗口 / 视觉辅助拆分。
- 第一轮不做 UIAutomation 控件候选；UIA/RTree 进入后续 Chapter E。
- 视觉候选只作为补充，不允许抢明确窗口、任务栏或显示器候选。

### 13.6 关于页第一轮设计

- 已确认：保留现有关于页入口，但重构当前关于页内容。
- 当前关于页里的旧内容不要了，后续实现时清理掉，不在旧内容上继续追加卡片。
- 新关于页采用轻量、优雅的产品信息卡。
- 必须展示：应用名称、真实版本号、作者 `犹少年`、GitHub 仓库按钮、联系方式。
- GitHub 按钮链接：`https://github.com/yoousn/screenshot-translator`。
- 联系方式：`gg1761229856@gmail.com`。
- 版本号优先从 `package.json` / Tauri 构建元数据读取，不允许继续硬编码 `v1.0.0`。
- 技术栈鸣谢、许可证、商业声明不进入第一轮，避免关于页变重或写入未经确认的信息。

### 13.7 推荐执行路线

默认执行路线已确认：

1. 先做 `Chapter 0`，只加基线打点，不改变行为。
2. 根据基线做 `A1`，移出低风险阻塞点，例如备份写盘、ready 门禁、show 结果确认。
3. 做 `A2`，只保留无闪不卡显示时序。
4. 做 `B-lite`，补齐普通保存状态机和轻量反馈。
5. 做 `D1`，补显示器候选、任务栏整体候选和坐标统一。
6. 做 `Chapter I`，重构关于页为轻量产品信息卡。
7. 是否进入 `C1` / `E`，必须由 Chapter 0/A1 后的数据和用户确认决定。

