# 截图翻译性能与速度极致优化设计规约 (2026-05-28)

本设计文档旨在为截图翻译系统提供大厂级别（如微信/QQ）的极速翻译优化方案，彻底解决“文字越多翻译越慢”的问题。

---

## 1. 架构总览

整个优化链路涵盖从 OCR 识别、文本重组、网络传输、缓存到渲染的每一个核心细节。

```mermaid
graph TD
    A[截图原始图像] --> B[PaddleOCR 本地识别]
    B --> C[空间相邻合并算法]
    C -->|生成| D[VirtualBlock 临时翻译块]
    D --> E[内存 LRU Cache + TTL 过滤]
    E -->|未命中词块| F[<SEG{idx}> 协议打包]
    F --> G[requests.Session 连接池单次网络请求]
    G --> H[正则匹配解析]
    H -->|缺失补齐| I[精准单句 ThreadPool Fallback]
    I --> J[缓存回填 & 写入 Cache]
    E -->|命中词块| J
    J --> K[union_bbox 擦除与重绘]
    K --> L[耗时打点 (控制台 + Header 摘要)]
```

---

## 2. 详细设计方案

### 2.1 耗时日志打点系统 (Step 01)
- **统计精度**：使用 `time.perf_counter()`，统一以毫秒（`ms`）为单位，保留 **2 位小数**。
- **展现形式**：
  1. **默认后端终端**：单次翻译结束后，打印清晰醒目的结构化分析报告，记录：
     - OCR 识别耗时及占比
     - 翻译网络请求耗时及占比
     - 图像重绘与渲染耗时及占比
     - 总耗时
  2. **HTTP 响应 Header 摘要**（支持前端 Tauri 实时监控卡顿）：
     - `X-Trace-Total-Ms`: 总耗时
     - `X-Trace-Ocr-Ms`: OCR 识别耗时
     - `X-Trace-Translate-Ms`: 翻译耗时
     - `X-Trace-Render-Ms`: 渲染重绘耗时
     - `X-Trace-Ocr-Blocks`: OCR 原始物理框数量
     - `X-Trace-Translate-Units`: 实际进入翻译的 Block/VirtualBlock 数量
     - `X-Trace-Cache-Hits`: 缓存命中数


---

### 2.2 翻译层真正 Batch 传输协议 (Step 02)
- **打包标记设计**：废弃易被翻译器理解或改写的 `[0]` 编号，采用 **低语义、不可翻译、固定结构、罕见** 的非自然语言标签：
  `"<SEG{idx}>文本"`（例如：`<SEG0>Hello<SEG1>World`）。
- **非换行依赖解析**：直接使用正则表达式匹配取值：
  ```python
  pattern = r"<SEG(\d+)>(.*?)(?=(?:<SEG\d+>|$))"
  ```
- **建立解析映射**：通过正则解析出 `idx -> text` 映射，并与原框序号进行对齐。
- **精准降级兜底**：如果翻译返回结果中缺失了部分 `idx` 的译文，系统**仅针对缺失的 idx** 发起并发线程池单句翻译，最大限度减少降级带来的额外延迟。

---

### 2.3 HTTP 连接复用与连接池 (Step 03)
- **底层改造**：在 `BaseTranslator` 层级中建立全局共享的 `requests.Session` 实例。
- **连接池参数**：
  ```python
  adapter = requests.adapters.HTTPAdapter(pool_connections=10, pool_maxsize=20)
  ```
- 所有翻译请求（Google、百度、LLM）均复用此 Session，保持持久的 TCP Keep-Alive 连接，尽可能减少重复的 TCP/TLS 握手开销（连接断开、代理变化或服务端主动关闭时仍会自动重建连接）。

---

### 2.4 内存 LRU 翻译缓存系统 (Step 04)
- **存储方案**：使用带有 24 小时过期机制（TTL）的**内存 LRU 缓存**（最大容量 `5000` 条）。
- **复合缓存键 (Composite Key)**：
  ```python
  key = (
      normalize_text(src_text),  # 对文本进行首尾 strip 并进行连续空白折叠（Collapsing）
      src_lang,
      dst_lang,
      translator_name,
      translator_version
  )
  ```
- **运行策略**：
  - 命中缓存的文本块立刻回填，耗时 0ms。
  - 未命中的文本块进入 batch 单请求打包，翻译成功后同步写入缓存。

---

### 2.5 空间相邻合并与虚拟块回填 (Step 05)
- **术语定义**：
  - `Block`：OCR 原始识别出的单行文本框。
  - `VirtualBlock`：由多个空间上高度相邻的 `Block` 合并而成的临时翻译块。
- **相邻合并判定条件**：
  - **同一行判定**：`abs(center_y1 - center_y2) <= 0.5 * avg_height`
  - **水平间距**：`gap_x <= 2.0 * avg_height`
  - **高度相近**：`max(h1, h2) / min(h1, h2) <= 1.5`
  - **合并硬限制**：每个 `VirtualBlock` 最多合并 **6** 个 `Block`，且合并后的字符总长度 **<= 80**。
- **重绘回填逻辑**：
  - 翻译前生成 `VirtualBlock`，并保留其所有子 `Block` 以便降级。
  - 翻译时只发送 `VirtualBlock`。
  - 翻译返回后，**绝不拆分**，直接计算出 `VirtualBlock` 内所有子 `Block` 的 `union_bbox`（外接矩形边界框），对该大框进行整体背景擦除与译文重绘。防止中英文长短不一导致文字挤压或错位。

## 3. 分阶段实施计划

按照如下优先级顺序逐步落地与验证：

- **Phase 1：只加耗时日志 + Header，不改业务逻辑**
  - 在 `ImageProcessor.process_and_draw` 中进行高精度计时打点。
  - 在 `server/app.py` 中捕获统计信息，格式化输出至控制台，并写入 `X-Trace-` 响应 Header。

- **Phase 2：加内存 LRU Cache**
  - 实现带 TTL (24h) 和容量限制 (5000) 的内存缓存。
  - 实现复合键标准化函数 `normalize_text` (strip + 连续空白折叠)。
  - 在翻译路由前拦截命中，未命中翻译后填入缓存。

- **Phase 3：加 <SEG> batch + 精准缺失 fallback**
  - 弃用原有的 Google 翻译多线程，升级为基于 `<SEG{idx}>` 协议的单次批量网络请求。
  - 实现正则提取 `idx -> text` 映射与原框对齐逻辑。
  - 增加对缺失 tag 的精准 ThreadPool 降级补齐机制。

- **Phase 4：加 VirtualBlock 空间合并**
  - 实现空间位置行对齐判定、水平间距、高度比值及数量/长度硬限制算法。
  - 翻译前重组生成 `VirtualBlock`，回填时计算 `union_bbox` 并以大框整体擦除重绘。

- **Phase 5：基准测试与验证**
  - 使用不同文本框数量（10/30/80）的测试图片进行全链路验证，输出结构化时延及缓存命中对比报告。

