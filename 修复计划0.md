# YSN 截图翻译 — Bug 修复与改进计划

> 本文档基于对仓库现有实现的实际审查（`server/`、`tauri-client/src/`、`tauri-client/src-tauri/src/`、配置与依赖）整理而成。
> 每条问题包含：**触发条件 / 复现线索**、**受影响文件**、**改动方向**、**预计收益**、**优先级（高 / 中 / 低）**。
>
> 审查范围：构建/运行、前端交互、后端接口、依赖与配置、测试与校验、日志与错误处理、边界情况、可新增的业务功能。

---

## 目录

1. [严重 Bug / 一致性问题（高优先级为主）](#一严重-bug--一致性问题)
2. [稳定性与并发问题](#二稳定性与并发问题)
3. [安全问题](#三安全问题)
4. [性能优化](#四性能优化)
5. [代码可维护性与工程化](#五代码可维护性与工程化)
6. [现有页面/组件的小幅交互优化](#六现有页面组件的小幅交互优化)
7. [可新增的业务功能（大厂同款）](#七可新增的业务功能大厂同款)
8. [优先级总览表](#八优先级总览表)

---

## 一、严重 Bug / 一致性问题

### 1.1 翻译通道存在"双轨配置"，前后端状态不一致 【高】 [已修复]

**修复记录 (2026-05-29)**
- server/app.py: add /api/config/save and /api/config/current.
- Settings.tsx: save server channel config before local config.


**触发条件 / 复现线索**
- 在「系统设置 → 活动翻译信道」把下拉切到「百度翻译」，点击右上角「保存设置」（但**不点**通道卡片内的「测试连接并启用」），返回控制面板。
- 控制面板（`Dashboard.tsx` 读取 `config.channel`）会显示「百度翻译」，但实际发起 `/api/translate` 时，服务器用的仍是 `~/.screenshot-translator/config.yaml` 里的 `active_channel`（仍是上一次的值，例如 google）。

**根因**
- 客户端 `config.json`（`save_config`/`get_config`，camelCase：`channel`、`newApiBase`…）与服务器 `config.yaml`（`load_server_config`，snake_case：`active_channel`、`channels.new-api.base_url`…）是两套独立配置。
- 真正决定翻译通道的是**服务器端** `active_channel`，它**只在** `POST /api/config/test` 成功后被写入（见 `server/app.py` 的 `test_and_save_config`）。客户端表单里的 `channel` 字段保存到本地后**从未参与**翻译通道判定，只用于前端展示。

**受影响文件**
- `server/app.py`：`get_active_translator()`、`test_and_save_config()`
- `tauri-client/src/pages/Settings.tsx`：`onFinish()`、`testChannel()`
- `tauri-client/src/pages/Dashboard.tsx`：读取 `config.channel` 展示通道

**改动方向**
- 方案 A（推荐）：让「保存设置」直接把 `active_channel` 与各通道凭证一并提交到服务器（新增 `POST /api/config/save`，或复用 `config/test` 但允许跳过连通性校验），客户端本地只保留 `serverUrl` / `clientToken` / OCR 相关本地项。
- 方案 B：在 `Dashboard` / `App` 中通过新接口 `GET /api/config/current`（返回脱敏后的 `active_channel`）读取**服务器真实通道**来展示，避免显示与实际不符。
- 统一约定：凡是"服务器行为相关"的配置（通道、模型、密钥）以服务器 `config.yaml` 为准；凡是"客户端本地行为"（serverUrl、token、本地 OCR 路径、快捷键、开机自启）以 `config.json` 为准。文档中明确这条边界。

**预计收益**：消除"设置显示与实际翻译不一致"的核心困惑，是当前最容易让用户误解的问题。

---

### 1.2 自定义快捷键 `hotkey` 配置项完全不生效 【高】 [已修复]

**修复记录 (2026-05-29)**
- lib.rs: read config.json hotkey on startup and parse modifiers/key.
- lib.rs: add re_register_shortcut(hotkey) for hot updates after Settings save.
- Settings/App/Dashboard: validate hotkey and show current hotkey conflict errors.


**触发条件 / 复现线索**
- 在「系统设置 → 全局截图快捷键」里把 `Alt+A` 改成别的组合并保存。重启程序后快捷键依旧是 `Alt+A`，新值无任何作用。

**根因**
- `tauri-client/src-tauri/src/lib.rs` 的 `run()` 中把快捷键**硬编码**为 `Shortcut::new(Some(Modifiers::ALT), Code::KeyA)` 与 `Code::KeyT`，从不读取 `config.json` 的 `hotkey`。
- 前端 `Settings.tsx` 存了 `hotkey`，`Dashboard.tsx` 也展示 `config.hotkey || "Alt+A"`，造成"可配置"的假象。

**受影响文件**
- `tauri-client/src-tauri/src/lib.rs`：`run()` 中快捷键注册逻辑
- `tauri-client/src/pages/Settings.tsx`：`hotkey` 表单项

**改动方向**
- 启动时读取 `config.json.hotkey`，解析为 `Modifiers + Code` 后再 `on_shortcut`；解析失败回退到 `Alt+A` 并把错误写入 `AppShortcutStatus`。
- 提供一个 `#[tauri::command] re_register_shortcut(hotkey: String)`，保存设置后调用以**热更新**快捷键（先 `unregister` 旧的再 `register` 新的），无需重启。
- 若短期不实现，则最简修复：把 `Settings.tsx` 的 `hotkey` 输入框设为 `disabled` 并标注「当前固定为 Alt+A / Alt+T」，避免误导。

**预计收益**：要么补齐承诺的功能，要么消除误导，二选一都能提升可信度。

---

### 1.3 `/api/ocr` 绕过 OCR 推理锁，与翻译并发时可能崩溃 【高】 [已修复]

**修复记录 (2026-05-29)**
- server/app.py: /api/ocr now calls processor.run_ocr() and reuses the OCR inference lock.


**触发条件 / 复现线索**
- 在悬浮工具栏快速连续触发「识字」与「翻译 (Ctrl+Q)」，或多客户端同时打 OCR/翻译请求时，PaddleOCR 偶发崩溃或返回错乱结果。

**根因**
- `server/app.py` 的 `ocr_image()` 直接调用 `processor.ocr.ocr(img_cv, cls=True)`，**没有**走 `processor.run_ocr()`，因此绕过了 `image_processor.py` 中的 `self._ocr_infer_lock`。
- 而 `process_and_draw()` 内部是通过 `run_ocr()` 加锁的。两条路径并发执行同一个非线程安全的 PaddleOCR 实例会冲突。

**受影响文件**
- `server/app.py`：`ocr_image()`
- `server/image_processor.py`：`run_ocr()`（已有锁，直接复用即可）

**改动方向**
```python
# server/app.py，ocr_image 内：
# 旧： ocr_result = processor.ocr.ocr(img_cv, cls=True)
ocr_result = processor.run_ocr(img_cv, cls=True)   # 复用带 _ocr_infer_lock 的方法
```

**预计收益**：消除并发下的偶发崩溃/乱码，是稳定性硬伤。

---

### 1.4 LLM 翻译缓存 Key 不区分 model，切换模型后命中旧译文 【高】 [已修复]

**修复记录 (2026-05-29)**
- translator.py: cache_namespace() includes LLM host and model.
- image_processor.py: text-cache key includes new-api base_url and model.


**触发条件 / 复现线索**
- 用 `gemini-3.5-flash` 翻译过某段文本（已进缓存），随后在设置里把模型换成另一个大模型并启用，再翻译同样文本——返回的仍是旧模型的译文（命中缓存）。

**根因**
- `server/translator.py` 的 `BaseTranslator.translate_batch()` 计算缓存 key 时：
  ```python
  channel_name = self.__class__.__name__.lower().replace("translator", "")  # 仅 "llm"/"baidu"/"google"
  version = "1.0"  # 固定常量
  key = GLOBAL_TRANSLATE_CACHE.make_key(text, source_lang, target_lang, channel_name, version)
  ```
  对 LLM 而言，`base_url` 与 `model` 不进入 key，导致不同模型/中转共享同一缓存。

**受影响文件**
- `server/translator.py`：`BaseTranslator.translate_batch()`、`LLMTranslator`

**改动方向**
- 给 translator 增加一个 `cache_namespace()` 方法：`GoogleTranslator` 返回 `"google"`；`LLMTranslator` 返回 `f"llm:{self.model}"`（必要时含 `base_url` 的 host）；`BaiduTranslator` 返回 `"baidu"`。
- `translate_batch` 用 `cache_namespace()` 替换当前的 `channel_name`。
- `image_processor.py` 的 `_make_text_cache_key()` 同理：目前用 `active_channel`，应把 LLM 的 model 拼进去（否则换模型同样命中整组翻译缓存）。

**预计收益**：修复"换了模型却没生效"的隐性 Bug，避免用户误判模型质量。

---

### 1.5 后端默认端口（18090）与文档/部署端口（8318）不一致 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- server/app.py: default direct-run host/port changed to 0.0.0.0:8318 with env overrides.


**触发条件 / 复现线索**
- 直接 `python app.py` 启动时监听 `127.0.0.1:18090`（见 `app.py` 底部 `uvicorn.run(... port=18090 ...)`），而 `README.md` 与部署说明写的是 `--port 8318`、内网 `192.168.1.3:8318`。新同学按 `app.py` 跑起来后，客户端默认连 `8318` 连不上。

**受影响文件**
- `server/app.py`：`if __name__ == "__main__"` 块
- `README.md`：启动命令

**改动方向**
- 统一端口来源：从 `config.yaml` 或环境变量 `SS_TRANSLATOR_PORT` 读取，默认 `8318`，并让 `app.py` 与 README 命令保持一致。
- `app.py` 启动 host 也需注意：`127.0.0.1` 仅本机可访问，N100 对外服务应为 `0.0.0.0`（与 README 一致）。

**预计收益**：消除"按文档跑不起来"的上手障碍。

---

### 1.6 `Dashboard.tsx` 监听 `screenshot-captured` 是死代码 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- ScreenshotPage.tsx: emit screenshot-captured after confirmScreenshot(), so Dashboard preview listener is active.


**触发条件 / 复现线索**
- `Dashboard.tsx` 的 `useEffect` 里 `listen("screenshot-captured", ...)` 期望收到原生选区截图并塞进「接口测试」面板。但全仓库 Rust 侧（`lib.rs`）**从未** `emit("screenshot-captured", ...)`（只 emit `screenshot-mode` / `screenshot-updated`）。该回调永远不会触发。

**受影响文件**
- `tauri-client/src/pages/Dashboard.tsx`（监听方）
- `tauri-client/src-tauri/src/lib.rs`（缺少 emit）

**改动方向**
- 二选一：① 在 `confirmScreenshot`/截图完成路径上由 Rust `emit("screenshot-captured", base64)`，真正打通"截图→自动填入接口测试"；② 删除该死监听，避免误导后续维护者。

**预计收益**：减少死代码与认知负担，或补齐一条便捷链路。

---

### 1.7 `targetLang` 配置项形同虚设，目标语言全程硬编码为 `zh` 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- Settings.tsx: add targetLang selector.
- ScreenshotPage.tsx/lib.rs/app.py/translator.py: pass target_lang through local OCR, remote translate, and LLM prompts.


**触发条件 / 复现线索**
- `ScreenshotPage.tsx` 的 `Config` 接口里声明了 `targetLang`，但 `handleTranslate`/`/api/translate_text` 调用中目标语言写死 `"zh"`；后端 `process_and_draw` 调用 `translator_batch_fn(texts, ...)` 时也固定 `"auto" -> "zh"`。

**受影响文件**
- `server/app.py`：`translate_image()` 内 `translator_batch_fn`、`translate_text_endpoint`
- `tauri-client/src/pages/ScreenshotPage.tsx`：`handleTranslate()`
- `tauri-client/src/pages/Settings.tsx`：缺少目标语言选择项

**改动方向**
- Settings 增加「目标语言」下拉（zh/en/ja/ko…），存入 config；`/api/translate` 与 `/api/translate_text` 接收 `target_lang` 参数并透传给 translator。
- 详见新功能 [7.3 多目标语言](#73-多目标语言切换-中)。

**预计收益**：把"看起来支持多语言"变成真支持，扩大适用人群。

---

## 二、稳定性与并发问题

### 2.1 文本翻译组缓存命中但块数不符时退化为"单块大字" 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- image_processor.py: cache block-count mismatch becomes cache miss instead of merging into one large block.


**触发条件 / 复现线索**
- `image_processor.py` 的 `process_and_draw` 中，当 `text_cache` 命中但 `len(cached_texts) != len(line_blocks)` 时，会执行 `line_blocks = [self._merge_blocks(line_blocks)]`，把整张图所有文字框合并成**一个大框**重绘，排版明显劣化（一大坨居中文字）。

**受影响文件**
- `server/image_processor.py`：`process_and_draw()` 文本缓存命中分支

**改动方向**
- 缓存 value 同时存储 `blocks 的 rect 布局`，命中时校验布局是否一致；不一致直接判缓存 miss、走正常翻译流程，而不是强行合并成单块。
- 或把缓存粒度下沉到"单行文本"（复用 `GLOBAL_TRANSLATE_CACHE`），移除 `TextTranslationCache` 这层"整图级"缓存，避免块数不一致问题。

**预计收益**：避免缓存命中时偶发的排版崩坏。

---

### 2.2 LLM 分段翻译的 `<SEG{i}>` 正则可能被译文内容干扰 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- translator.py: replace XML-style <SEG{i}> markers with private-use markers \uE000{i}\uE001.
- translator.py: parse segment responses with exact index-set and match-count validation.
- translator.py: invalid/missing marker responses fall back to per-text translation instead of risking misplaced translations.
- tests/test_translator.py: cover literal <SEG2> text and missing-marker fallback.

### 2.3 前端长耗时翻译缺少超时/取消，loading 可能永久挂起 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- ScreenshotPage.tsx: image decode failure now rejects into catch and clears loading state.


**触发条件 / 复现线索**
- `ScreenshotPage.handleTranslate` 走 `invoke("api_translate")`（Rust 侧 reqwest 超时 60s）或本地 OCR 链路。`message.loading({ duration: 0 })` 在异常路径若未命中 catch（例如 `overlayImg.onerror` 内 `throw` 在异步回调里无法被外层 try/catch 捕获）时，loading 提示不会关闭。

**受影响文件**
- `tauri-client/src/pages/ScreenshotPage.tsx`：`handleTranslate()` 中 `overlayImg.onerror` 的 `throw`

**改动方向**
- `overlayImg.onerror` 改为直接 `message.error(...)` + `setIsTranslating(false)` + 关闭 loading，而不是 `throw`（异步回调里的 throw 不会被同步 try/catch 捕获）。
- 给所有 loading 提示设置兜底 `duration`，或在 `finally` 中统一 `message.destroy("translate")`。

**预计收益**：消除"翻译中…"永久卡住的体验问题。

---

### 2.4 OCR 后台预热线程异常时静默 + 首请求仍可能撞冷启动 【低】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- app.py: warmup 失败由 `print` 改为 `logger.error` 并输出完整 traceback。
- app.py: `/api/health` 返回 `ocr_ready` 字段，客户端可据此展示加载状态。

**触发条件 / 复现线索**
- `app.py` 启动后 `warm_up_ocr_async` 后台预热；若预热线程在 `_ensure_ocr()` 抛错（如模型下载失败），仅 `print` 一行，首个真实请求才暴露问题。

**受影响文件**
- `server/app.py`：`warm_up_ocr_async()`

**改动方向**
- 预热失败写入 `logger.error` 并记录可被 `/api/health` 暴露的 `ocr_ready` 状态（health 返回 `{"status":"ok","ocr_ready":processor.ocr_ready}`），便于客户端展示"模型加载中"。

**预计收益**：可观测性提升，便于排查 N100 上的模型加载问题。

---

## 三、安全问题

### 3.1 LLM 中转地址在"测试/实际翻译"时缺少 SSRF 校验 【高】 [已修复]

**修复记录 (2026-05-29)**
- server/security.py: add normalize_public_base_url() and reject private/loopback/reserved URLs.
- app.py/translator.py: apply URL validation in fetch_models, config save/test, and LLMTranslator init.


**触发条件 / 复现线索**
- `_validate_url()` 仅在 `fetch_models` 中调用。`test_and_save_config` 与实际翻译时实例化的 `LLMTranslator` 会向用户填写的 `base_url` 发起请求，**未做**私有/回环/保留地址校验。若服务暴露在公网，攻击者可借此探测内网（SSRF）。

**受影响文件**
- `server/app.py`：`test_and_save_config()`、`get_active_translator()`
- `server/translator.py`：`LLMTranslator.__init__`

**改动方向**
- 把 `_validate_url()` 抽到公共模块，在 `LLMTranslator` 实例化或发请求前统一校验 `base_url`（百度/谷歌为固定官方域名可豁免）。
- 可配置白名单域名（如仅允许 `api.yousn.me`）。

**预计收益**：闭合 SSRF 风险面，尤其在外网 `ocr.yousn.me` 暴露的前提下。

---

### 3.2 默认 token 硬编码在源码中 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- config.py: 首次生成配置时使用 `secrets.token_urlsafe(32)` 生成随机 token，不再硬编码。
- config.py: 新增 `_apply_env_overrides()` 支持 `SS_TRANSLATOR_TOKEN` 环境变量覆盖。
- app.py: 启动时打印当前 client_token 到控制台，供运维人员在客户端配置。
- README.md: 移除明文 token，改为说明首次启动自动生成。

**触发条件 / 复现线索**
- `server/config.py` 的 `_default_config()` 写死 `"client_token": "ysn-screenshot-translator-token-666"`，且已进入仓库与 README。任何拿到源码的人都知道默认令牌。

**受影响文件**
- `server/config.py`：`_default_config()`
- `README.md`：明文展示 token

**改动方向**
- 首次启动时随机生成 token（`secrets.token_urlsafe(32)`）写入 `config.yaml`，并在服务器日志/控制台打印一次供客户端填写；移除 README 中的明文。
- 支持从环境变量 `SS_TRANSLATOR_TOKEN` 覆盖。

**预计收益**：避免默认弱口令导致未授权调用。

---

### 3.3 CORS 与 CSP 策略偏宽松 【低】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- 在 `server/app.py` 中将 `allow_origins=["*"]` 限制为 `["http://localhost:1420", "http://127.0.0.1:1420", "tauri://localhost", "https://tauri.localhost"]`。
- 将 `allow_methods` 限制为 `["GET", "POST", "OPTIONS"]`。

**受影响文件**
- `server/app.py`：`add_middleware(CORSMiddleware, ...)`
- `tauri-client/src-tauri/tauri.conf.json`：`app.security.csp`

**预计收益**：纵深防御，降低 webview 注入风险。

---

## 四、性能优化

### 4.1 前端 `renderTranslatedBlocks` 逐像素 `getImageData` 未开 `willReadFrequently` 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- ScreenshotPage.tsx: create renderTranslatedBlocks canvas context with willReadFrequently=true.


**触发条件 / 复现线索**
- `ScreenshotPage.tsx` 的 `renderTranslatedBlocks` 对每个块 4 个角点各调用一次 `ctx.getImageData(cx, cy, 1, 1)`，块多时浏览器会发 `Canvas2D: Multiple readback operations using getImageData are slow` 警告且确实变慢。

**受影响文件**
- `tauri-client/src/pages/ScreenshotPage.tsx`：`renderTranslatedBlocks()`

**改动方向**
- 创建 context 时 `canvas.getContext("2d", { willReadFrequently: true })`。
- 进一步：一次性 `getImageData(0,0,w,h)` 拿到整图像素，后续按坐标在 `Uint8ClampedArray` 里直接索引采样，避免多次 readback。

**预计收益**：本地重绘（本地 OCR 模式）在多文本块时更顺滑。

---

### 4.2 大截图经 base64 走 `invoke` 往返，体积与编解码开销大 【低】 [已修复]

**触发条件 / 复现线索**
- 全屏 4K 截图 PNG → base64（膨胀约 33%）在 Rust↔WebView 之间多次传递（`get_fullscreen_image`、`capture_region`、`api_translate`）。大图时序列化/反序列化耗时可观。

**受影响文件**
- `tauri-client/src-tauri/src/lib.rs`：`get_fullscreen_image`、`capture_region`
- `tauri-client/src/pages/ScreenshotPage.tsx`

**改动方向**
- 全屏底图传输用 JPEG（已是 jpeg）即可；裁剪区考虑直接在 Rust 侧完成"裁剪→请求服务器→返回结果"的闭环，减少 base64 跨界次数。
- 或评估使用 Tauri 的 `Channel`/二进制 IPC 传字节而非 base64 字符串。

**预计收益**：降低大图场景延迟与内存峰值。

---

### 4.3 `get_active_translator` 每请求新建实例（影响小，可观测性可加） 【低】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- app.py: 按 `channel + 凭证 hash` 缓存 translator 实例，配置变更时自动失效。
- app.py: 通道信息由 `print` 降级为 `logger.debug`，仅首次创建时 `logger.info`。

**触发条件 / 复现线索**
- 每次 `/api/translate` 都 `new` 一个 translator（共享 `_shared_session`，开销不大）。但每次都 `print` 通道信息，高频时日志噪音大。

**受影响文件**
- `server/app.py`：`get_active_translator()`

**改动方向**
- 按 `active_channel + 凭证 hash` 缓存 translator 实例；把 `print` 降级为 `logger.debug`。

**预计收益**：减少日志噪音，轻微降低开销。

---

## 五、代码可维护性与工程化

### 5.1 测试与调试产物混入仓库、目录结构混乱 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- 将 `server/test_*.py` 全部移动至 `server/tests/` 统一管理。
- 添加 `server/pytest.ini` 修复路径导入上下文。
- 从根目录删除了零散的 `.png` 产物，并更新了 `.gitignore` 防止未来误提。

**触发条件 / 复现线索**
- `server/` 根目录散落大量 `test_*.py`（`test_app_timing.py`、`test_batch_seg.py`、`test_cache*.py` 等）与正式的 `server/tests/`；同时有 `diag_local.png`、`test_result.png` 等二进制产物被提交。

**受影响文件**
- `server/test_*.py`、`server/*.png`、`server/tests/`

**改动方向**
- 将根目录散落的 `test_*.py` 收敛到 `server/tests/`，统一用 `pytest` 组织；`.png` 产物加入 `.gitignore` 并从版本库移除。
- 增加 `server/pytest.ini` 或 `pyproject.toml` 配置测试发现路径。

**预计收益**：仓库整洁、测试可一键运行（`pytest server/tests`）。

---

### 5.2 后端大量 `print` 而非 `logging` 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- app.py: 全部 `print` 替换为 `logger.info/debug/warning/exception`（仅保留启动 token 打印）。
- image_processor.py: `print` 替换为 `logger.warning`。
- 耗时报告改用 `logger.info`，异常用 `logger.exception`。

**触发条件 / 复现线索**
- `app.py`/`image_processor.py` 内有大量 `print(...)`（耗时报告、通道信息、异常）。生产环境无法按级别控制、无时间戳、无法接入日志文件轮转。

**受影响文件**
- `server/app.py`、`server/image_processor.py`

**改动方向**
- 统一用 `logging`（已在 `translator.py`/`config.py` 用了 logger）。耗时报告用 `logger.info`，异常 `logger.exception`。
- 用 `debug_trace` 配置项控制是否输出耗时表（目前 `app.py` 默认 `True`，`config.py` 默认 `False`，二者不一致，需对齐）。

**预计收益**：可观测性与可运维性提升，生产可静默。

---

### 5.3 `debug_trace` 默认值前后矛盾 【低】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- app.py: `get_config().get("debug_trace", True)` → `get_config().get("debug_trace", False)`，与 config.py 默认值对齐。

**触发条件 / 复现线索**
- `app.py` 中 `get_config().get("debug_trace", True)`（缺省 True），而 `config.py` 的 `_default_config()` 里 `"debug_trace": False`。读取逻辑与默认配置不一致，行为取决于配置文件是否已生成。

**受影响文件**
- `server/app.py`、`server/config.py`

**改动方向**
- 统一缺省值（建议生产默认 `False`），并集中到一处定义。

---

### 5.4 README 与实际结构/命令不符 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- 更新 `README.md` 的项目结构目录，修正 `client/` 为 `tauri-client/`，`main.rs` 为 `src-tauri/src/lib.rs` 等。
- 清理 `.gitignore` 中失效的旧规则。
**触发条件 / 复现线索**
- `README.md` 的项目结构写的是 `client/`、`main.rs`、`config.*`，实际是 `tauri-client/`、`src-tauri/src/lib.rs`、`config.json`/`config.yaml`。`.gitignore` 里也仍保留 `/client/...`、`server/config.yaml`、`/server/ocr/` 等旧路径。
- 启动端口、目录名都与现状有出入。

**受影响文件**
- `README.md`、`.gitignore`

**改动方向**
- 更新 README 的目录树、启动命令（端口对齐）、依赖列表（与 `requirements.txt` 对齐：实际还需 `paddlepaddle`、`python-multipart`）。
- 清理 `.gitignore` 中的 `client/` 残留，补充 `server/*.png`、`server/.venv/`。

**预计收益**：新成员可按文档一次性跑通。

---

### 5.5 PaddleOCR 依赖版本风险 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- 在 `README.md` 安装依赖段落加入了针对 `paddleocr` 2.x 版本系列的黄框警示说明，防止后续误升级 3.x 导致不兼容。

**触发条件 / 复现线索**
- `requirements.txt` 约束 `paddleocr>=2.7.0,<3.0.0`、`paddlepaddle>=2.5.2,<3.0.0`。代码使用的 `PaddleOCR(..., det_db_box_thresh=..., show_log=False)` 与 `ocr(img, cls=True)` 是 2.x API；PaddleOCR 3.x 已改签名。约束本身正确，但需在 README 注明"必须 2.x"，否则有人手动升级会崩。
- `numpy<2.0.0` 与较新 paddle 的兼容也需固定（已 pin，保留）。

**受影响文件**
- `server/requirements.txt`、`README.md`

**改动方向**
- 在 README/requirements 注释中明确版本锁定原因；考虑提供 `requirements.lock` 或容器镜像，保证 N100 可复现部署。

**预计收益**：避免依赖漂移导致的部署事故。

---

## 六、现有页面/组件的小幅交互优化

### 6.1 `History` 页面全是 mock 数据，存在误导 【中】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- 删除了 `History.tsx` 中的虚假硬编码数据，替换为 Ant Design 的 `Empty` 状态提示。
- 将文案更新为“真实历史翻译记录功能即将上线”，避免对当前用户产生误导。

**触发条件 / 复现线索**
- `History.tsx` 用 `mockHistory` 写死 5 条记录；`handleClearHistory` 只弹「暂无可清理的历史数据」。用户会以为有真实历史功能。

**受影响文件**
- `tauri-client/src/pages/History.tsx`

**改动方向**
- 见新功能 [7.1 真实翻译历史](#71-真实翻译历史持久化-高)。短期内若不做持久化，应在页面加 `Empty`/“演示数据”标识，避免误导。

**预计收益**：界面文案专业度。

---

### 6.3 服务在线状态在 App 与 Dashboard 各自独立轮询 【低】 [已修复] [已修复]

**修复记录 (2026-05-29)**
- 移除了 `Dashboard.tsx` 内部的 `checkServer` 方法与状态。
- 在顶层 `App.tsx` 的 `checkStatus` 中统一轮询，并计算出 `responseTime`。
- 通过 Props `serverStatus`, `responseTime`, `onRefreshStatus` 透传至 `Dashboard.tsx`，避免了切换组件和初始化时的重复网络开销。-client/src/App.tsx`、`tauri-client/src/pages/Dashboard.tsx`

**改动方向**
- 抽一个 `useServerStatus(serverUrl)` hook（或用 SWR 共享），统一健康检查与响应时延展示。

**预计收益**：状态一致、减少重复请求与代码。

---

## 七、可新增的业务功能（大厂同款）

### 7.1 真实翻译历史持久化 【高】 [已修复]

**修复记录 (2026-05-29)**
- src-tauri/src/lib.rs: 新增 get_history, dd_history, clear_history 命令，持久化至 ppDataDir/history.json。
- ScreenshotPage.tsx: 在翻译完成后将翻译结果与时间等耗时写入 history。
- Dashboard.tsx: 在接口测试翻译成功后也记录 history。
- History.tsx: 替换 mock 数据，对接真实后端的 history 命令。

**现状**：`History.tsx` 全 mock，无任何持久化。
**建议**
- 每次翻译/OCR 成功后，由 Rust 把"缩略图 + 原文 + 译文 + 通道 + 耗时 + 时间戳"写入本地（`app_data_dir()/history.json` 或 SQLite via `tauri-plugin-sql`）。
- `History` 页面读取真实记录，支持：缩略图预览、**一键重翻**、复制译文、删除单条 / 清空、按通道筛选。
**受影响文件**：`tauri-client/src-tauri/src/lib.rs`（新增 `save_history`/`load_history` 命令）、`tauri-client/src/pages/History.tsx`
**预计收益**：从"演示"变"可用"，对标 Snipaste/QQ 截图的历史能力。

### 7.2 翻译结果"原文 / 译文"对照侧栏 【中】 [已修复]

**现状**：截图翻译只在图上重绘，看不到纯文本对照。
**建议**：翻译完成后，在选区侧弹出可复制的"原文 → 译文"列表（复用现有 `/api/translate_text` 即可拿到文本），支持整体复制译文。
**受影响文件**：`tauri-client/src/pages/ScreenshotPage.tsx`
**预计收益**：满足"既要图上译文又要纯文本"的高频诉求。

### 7.3 多目标语言切换 【中】 [已修复]

见 [1.7](#17-targetlang-配置项形同虚设目标语言全程硬编码为-zh-中)。在设置与悬浮栏提供目标语言下拉（中/英/日/韩等），后端透传 `target_lang`。
**预计收益**：覆盖中译外、外译外场景。

### 7.4 钉图（Pin）悬浮窗 【中】 [已修复]

**现状**：README 宣称"翻译结果可置顶钉图为独立悬浮窗口"，但代码中**未实现** Pin 窗口（只有复制/保存）。
**建议**：新增 `pin_image(base64)` 命令，创建一个无边框、`always_on_top`、可拖动缩放的小窗展示译图；对标 Snipaste 贴图。
**受影响文件**：`tauri-client/src-tauri/src/lib.rs`（新窗口）、新 `PinWindow` 前端页面、`main.tsx` 路由分发
**预计收益**：兑现 README 承诺，是截图工具的招牌功能。

### 7.5 OCR 低置信度过滤与文字方向/竖排支持 【低】 [已修复]

**建议**：`/api/ocr` 与重绘流程增加按 `confidence` 阈值过滤（可配置），减少把噪点识别成乱码后重绘的情况；评估竖排/旋转文本。
**受影响文件**：`server/image_processor.py`、`server/app.py`
**预计收益**：提升嵌字结果质量。

### 7.6 截图标注（箭头/矩形/马赛克/文字）【低】 [已修复]

**建议**：在选区确认后提供轻量标注工具条（对标飞书/钉钉截图）。可作为后续大版本。
**受影响文件**：`tauri-client/src/pages/ScreenshotPage.tsx`

---

## 八、优先级总览表

| 编号 | 问题 / 功能 | 类型 | 优先级 | 状态 |
| --- | --- | --- | --- | --- |
| 7.1 | 真实翻译历史持久化 | 新功能 | 高 | ✅ 已修复 |
| 1.5 | 默认端口 18090 与文档 8318 不符 | 一致性 | 中 | ✅ 已修复 |
| 1.6 | `screenshot-captured` 死代码 | 可维护性 | 中 | ✅ 已修复 |
| 1.7 | `targetLang` 形同虚设 | Bug | 中 | ✅ 已修复 |
| 2.1 | 文本缓存命中退化为单块 | 稳定性 | 中 | ✅ 已修复 |
| 3.1 | 服务端解析公网图片的 SSRF | 安全 | 高 | ✅ 已修复 |
| 3.2 | 默认 token 硬编码 | 安全 | 中 | ✅ 已修复 |
| 3.3 | CORS/CSP 策略偏宽松 | 安全 | 低 | ✅ 已修复 |
| 4.1 | `getImageData` 未开 willReadFrequently | 性能 | 中 | ✅ 已修复 |
| 5.1 | 测试/产物混入仓库、结构乱 | 工程化 | 中 | ✅ 已修复 |
| 5.2 | 后端 print 而非 logging | 工程化 | 中 | ✅ 已修复 |
| 5.4 | README 与实际不符 | 文档 | 中 | ✅ 已修复 |
| 5.5 | PaddleOCR 版本风险说明 | 工程化 | 中 | ✅ 已修复 |
| 6.1 | History mock 数据误导 | 交互 | 中 | ✅ 已修复 |
| 7.2 | 原文/译文对照侧栏 | 新功能 | 中 | ❌ 已修复 |
| 7.3 | 多目标语言切换 | 新功能 | 中 | ❌ 已修复 |
| 7.4 | 钉图 Pin 悬浮窗 | 新功能 | 中 | ❌ 已修复 |
| 2.4 | OCR 预热失败静默 | 稳定性 | 低 | ✅ 已修复 |

| 4.2 | 大图 base64 往返开销 | 性能 | 低 | ❌ 已修复 |
| 4.3 | translator 每请求新建+日志噪音 | 性能 | 低 | ✅ 已修复 |
| 5.3 | `debug_trace` 默认值矛盾 | 工程化 | 低 | ✅ 已修复 |
| 6.2 | Settings 页面无效字符 `&nbsp;` | 交互 | 中 | ✅ 已修复 |
| 6.3 | 健康检查重复轮询 | 交互 | 低 | ✅ 已修复 |
| 6.4 | 选区工具栏边缘溢出 | 交互 | 低 | ❌ 已修复 |
| 7.5 | OCR 置信度过滤/竖排 | 新功能 | 低 | ❌ 已修复 |
| 7.6 | 截图标注工具 | 新功能 | 低 | ❌ 已弃用 |

---

### 建议的执行顺序（迭代规划）

1. ~~**第一批（修硬伤）**：1.3 推理锁、1.4 缓存 key、1.1 通道一致性、3.1 SSRF、2.3 loading 挂起。~~ ✅ 全部完成
2. ~~**第二批（补承诺/上手）**：1.2 快捷键生效、1.5 端口对齐、5.4 README、7.1 真实历史~~、7.4 钉图。 ⏳ 部分完成
3. ~~**第三批（体验/质量）**：1.7 多语言、~~7.2 对照侧栏、~~2.1/2.2 缓存与分段健壮性、4.1 性能、5.1/5.2 工程化。~~ ⏳ 部分完成
4. **第四批（增强）**：其余低优先级项与 7.5/7.6 新功能。

