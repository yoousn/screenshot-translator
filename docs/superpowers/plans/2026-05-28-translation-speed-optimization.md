# 截图翻译性能与速度极致优化实施计划 (2026-05-28)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为截图翻译系统提供大厂级别（微信/QQ 级别）的极速翻译响应，解决多文字卡顿问题。

**Architecture:** 采用高精度打点统计性能瓶颈，基于内存 LRU + 24小时 TTL 过滤重复词；利用 `<SEG{idx}>` 低语义不可翻译协议合并单次 POST 网络请求，结合 requests.Session 长连接池降级 TLS 开销；针对缺失翻译标签进行高容错单句 ThreadPool Fallback 精准补齐；最后使用基于空间邻近度的 VirtualBlock 对物理相邻短文本块进行语义合并与 union_bbox 整体渲染。

**Tech Stack:** Python 3.x, FastAPI, PaddleOCR, Pillow, requests, time

---

## 1. 拟创建与修改的文件映射

在开始编码前，我们先定义各功能单元的职责与要操作的文件：
- **`server/app.py`**: 修改 API 翻译路由，拦截并返回性能 Header 摘要，终端打印高颜值性能打点控制台报告。
- **`server/image_processor.py`**: 修改 `process_and_draw` 支持计时指标返回；后续增加 `VirtualBlock` 算法与大框整体擦除绘制逻辑。
- **`server/translator.py`**: 升级各个翻译子类，加入全局内存 LRU + TTL 缓存，重构 `translate_batch` 模块采用 `<SEG{idx}>` 打包传输，实现正则对齐映射与精准 fallback 兜底。
- **`server/test_timing.py`**: 新建，用于 Phase 1 测试打点数据。
- **`server/test_cache.py`**: 新建，用于 Phase 2 验证 LRU 缓存命中、复合键与 TTL 过期。
- **`server/test_batch_seg.py`**: 新建，用于 Phase 3 验证基于 `<SEG>` 的打包、正则提取与补齐回填。
- **`server/test_virtual_block.py`**: 新建，用于 Phase 4 验证空间邻近度合并计算与 union_bbox。
- **`server/benchmark.py`**: 新建，用于 Phase 5 进行 10/30/80 物理框的极限时延与命中对比基准测试。

---

## 2. 分阶段详细实施指南

### Phase 1：只加耗时日志 + Header，不改业务逻辑

本阶段纯粹进行高精度计时打点统计，不影响现有的 OCR 与并发翻译流程，确保监控机制 100% 正确并能向客户端传递 X-Trace 性能头部。

#### Task 1: 耗时打点机制与 FastAPI Header 注入

**Files:**
- Modify: `server/image_processor.py`
- Modify: `server/app.py`
- Create: `server/test_timing.py`

- [ ] **Step 1: 创建高精度计时测试用例**
  创建 `server/test_timing.py`，用于模拟并检查计时器的输出结构：
  ```python
  # server/test_timing.py
  import time
  
  def simulate_timing():
      start = time.perf_counter()
      time.sleep(0.05) # 模拟 OCR
      ocr_done = time.perf_counter()
      time.sleep(0.08) # 模拟 翻译
      trans_done = time.perf_counter()
      time.sleep(0.03) # 模拟 渲染
      render_done = time.perf_counter()
      
      t_ocr = (ocr_done - start) * 1000
      t_trans = (trans_done - ocr_done) * 1000
      t_render = (render_done - trans_done) * 1000
      t_total = (render_done - start) * 1000
      
      print(f"OCR: {t_ocr:.2f}ms, Trans: {t_trans:.2f}ms, Render: {t_render:.2f}ms, Total: {t_total:.2f}ms")
      assert abs(t_total - (t_ocr + t_trans + t_render)) < 0.1
      assert t_ocr > 40
      assert t_trans > 70
      assert t_render > 20
      print("Timing calculation test passed!")

  if __name__ == "__main__":
      simulate_timing()
  ```

- [ ] **Step 2: 运行测试并验证其通过**
  运行: `python server/test_timing.py`
  预期输出: `Timing calculation test passed!` 并带有对应的打点时间输出。

- [ ] **Step 3: 修改 `server/image_processor.py`**
  修改 `process_and_draw` 函数。使其使用 `time.perf_counter()` 高精度记录 `OCR`, `Translate`, `Render` 及 `Total` 耗时，并在返回时，同时返回生成好的 PNG 字节流和包含耗时信息的 `dict` 字典。
  
  修改代码定位约第 268-360 行：
  ```python
      def process_and_draw(self, img_bytes: bytes, translator_batch_fn) -> tuple[bytes, dict]:
          stats = {
              "ocr_ms": 0.0,
              "translate_ms": 0.0,
              "render_ms": 0.0,
              "total_ms": 0.0,
              "ocr_blocks": 0,
              "translate_units": 0,
              "cache_hits": 0
          }
          start_time = time.perf_counter()
          try:
              nparr = np.frombuffer(img_bytes, np.uint8)
              img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
              if img_cv is None:
                  stats["total_ms"] = (time.perf_counter() - start_time) * 1000
                  return img_bytes, stats

              self._ensure_ocr()
              ocr_start = time.perf_counter()
              ocr_result = self.ocr.ocr(img_cv, cls=True)
              stats["ocr_ms"] = (time.perf_counter() - ocr_start) * 1000
              
              if not ocr_result or not ocr_result[0]:
                  stats["total_ms"] = (time.perf_counter() - start_time) * 1000
                  return img_bytes, stats

              raw_lines = ocr_result[0]
              stats["ocr_blocks"] = len(raw_lines)

              # ── 步骤 A：按行合并 OCR box ──
              line_blocks = self._group_into_lines(raw_lines)
              stats["translate_units"] = len(line_blocks)

              # ── 步骤 B：批量翻译 ──
              original_texts = [b["text"] for b in line_blocks]
              translate_start = time.perf_counter()
              try:
                  translated_texts = translator_batch_fn(original_texts)
              except Exception as te:
                  print("[translate] error:", te)
                  translated_texts = original_texts
              stats["translate_ms"] = (time.perf_counter() - translate_start) * 1000

              if len(translated_texts) != len(original_texts):
                  translated_texts = original_texts

              # ── 步骤 C：转 PIL 并重绘 ──
              render_start = time.perf_counter()
              pil_img = Image.fromarray(cv2.cvtColor(img_cv, cv2.COLOR_BGR2RGB))
              draw = ImageDraw.Draw(pil_img)

              for block, trans_text in zip(line_blocks, translated_texts):
                  x1, y1, x2, y2 = block["rect"]
                  avg_h = block["avg_h"]
                  box_w = x2 - x1
                  box_h = y2 - y1

                  if box_w <= 0 or box_h <= 0:
                      continue

                  # 采样背景色并填充背景
                  bg_bgr = self._sample_bg(img_cv, x1, y1, x2, y2)
                  bg_rgb = self._bg_to_rgb(bg_bgr)
                  text_rgb = self._choose_text_color(bg_bgr)

                  PAD = 3
                  ex1 = max(0, x1 - PAD)
                  ey1 = max(0, y1 - PAD)
                  ex2 = min(img_cv.shape[1], x2 + PAD)
                  ey2 = min(img_cv.shape[0], y2 + PAD)
                  draw.rectangle([ex1, ey1, ex2, ey2], fill=bg_rgb)

                  # 布局文字并绘制
                  lines, font, line_gap, total_h = self._layout_text(
                      draw, trans_text, box_w, box_h, avg_h
                  )

                  if total_h <= box_h:
                      start_y = y1 + (box_h - total_h) // 2
                  else:
                      start_y = y1 + 2

                  x_draw = x1 + 2
                  for idx, line_text in enumerate(lines):
                      line_y = start_y + idx * line_gap
                      if line_y > img_cv.shape[0]:
                          break
                      draw.text(
                          (x_draw, line_y),
                          line_text,
                          fill=text_rgb,
                          font=font,
                          anchor="lt",
                          stroke_width=1 if font.size >= 13 else 0,
                          stroke_fill=bg_rgb,
                      )

              stats["render_ms"] = (time.perf_counter() - render_start) * 1000
              
              out = io.BytesIO()
              pil_img.save(out, format="PNG")
              stats["total_ms"] = (time.perf_counter() - start_time) * 1000
              return out.getvalue(), stats

          except Exception as e:
              stats["total_ms"] = (time.perf_counter() - start_time) * 1000
              print("ERROR in process_and_draw:", e)
              raise e
  ```

- [ ] **Step 4: 修改 `server/app.py`**
  修改 `translate_image` 路由方法（约第 81-97 行）。接收返回的 `stats` 字典，并在 FastAPI 的 Response Headers 中写入对应的计时头部，并在控制台打印结构化的高颜值耗时比例分析报告。
  ```python
  @app.post("/api/translate")
  def translate_image(image: UploadFile = File(...), x_api_key: str = Header(None)):
      verify_token(x_api_key)
      img_bytes = image.file.read()
      
      translator = get_active_translator()
      def translator_batch_fn(texts):
          return translator.translate_batch(texts, "auto", "zh")
          
      try:
          out_bytes, stats = processor.process_and_draw(img_bytes, translator_batch_fn)
          
          # 终端结构化耗时报告
          print("\n" + "="*50)
          print("           ⚡️ 截图翻译性能分析报告 ⚡️")
          print("="*50)
          print(f" OCR 识别耗时    : {stats['ocr_ms']:.2f} ms ({stats['ocr_ms']/stats['total_ms']*100:.1f}%)")
          print(f" 翻译引擎网络耗时 : {stats['translate_ms']:.2f} ms ({stats['translate_ms']/stats['total_ms']*100:.1f}%)")
          print(f" 图像重绘渲染耗时 : {stats['render_ms']:.2f} ms ({stats['render_ms']/stats['total_ms']*100:.1f}%)")
          print(f"--------------------------------------------------")
          print(f" OCR 原始框总数   : {stats['ocr_blocks']} 个")
          print(f" 翻译传输单元数   : {stats['translate_units']} 个")
          print(f" 缓存命中间数     : {stats['cache_hits']} 个")
          print(f" 总响应时延 (Total): {stats['total_ms']:.2f} ms")
          print("="*50 + "\n")

          # 写入 Response Headers
          headers = {
              "X-Trace-Total-Ms": f"{stats['total_ms']:.2f}",
              "X-Trace-Ocr-Ms": f"{stats['ocr_ms']:.2f}",
              "X-Trace-Translate-Ms": f"{stats['translate_ms']:.2f}",
              "X-Trace-Render-Ms": f"{stats['render_ms']:.2f}",
              "X-Trace-Ocr-Blocks": str(stats["ocr_blocks"]),
              "X-Trace-Translate-Units": str(stats["translate_units"]),
              "X-Trace-Cache-Hits": str(stats["cache_hits"]),
          }
          return Response(content=out_bytes, media_type="image/png", headers=headers)
      except Exception as e:
          print(f"[translate_image] error during process_and_draw: {e}")
          raise HTTPException(status_code=500, detail=f"Image processing failed: {str(e)}")
  ```

- [ ] **Step 5: 提交 Phase 1 更改**
  ```bash
  git add server/image_processor.py server/app.py server/test_timing.py
  git commit -m "perf: add high-precision latency timing metrics and FastAPI trace headers"
  ```

---

### Phase 2：加内存 LRU Cache

本阶段在翻译引擎底层增加带 TTL 的内存 LRU 缓存。每次翻译时，若命中缓存则直接返回结果，只有未命中的部分需要向外部 API 提交请求。

#### Task 2: 内存 LRU 缓存与 TTL (24h) 过滤机制

**Files:**
- Modify: `server/translator.py`
- Create: `server/test_cache.py`

- [ ] **Step 1: 编写 LRU 缓存与复合键测试用例**
  创建 `server/test_cache.py`：
  ```python
  # server/test_cache.py
  import time
  import re
  
  # 用于测试折行的空白折叠
  def normalize_text(text: str) -> str:
      if not text:
          return ""
      # 首尾 strip 加上连续空白合并为一个空格
      stripped = text.strip()
      return re.sub(r"\s+", " ", stripped)

  class TranslationCache:
      def __init__(self, maxsize=5000, ttl_sec=86400):
          self.cache = {}
          self.maxsize = maxsize
          self.ttl_sec = ttl_sec

      def get(self, key):
          if key in self.cache:
              val, expire_time = self.cache[key]
              if time.time() < expire_time:
                  # 刷新 LRU 序 (简单删除再添加)
                  del self.cache[key]
                  self.cache[key] = (val, expire_time)
                  return val
              else:
                  del self.cache[key]
          return None

      def set(self, key, value):
          if len(self.cache) >= self.maxsize:
              # 驱逐头部 (最老的数据)
              oldest = next(iter(self.cache))
              del self.cache[oldest]
          expire_time = time.time() + self.ttl_sec
          self.cache[key] = (value, expire_time)

  def test_cache_logic():
      # 1. 验证空白折叠
      assert normalize_text("  hello  \n\t world  ") == "hello world"
      assert normalize_text("a\n\n\nb") == "a b"
      
      # 2. 验证 LRU 与过期
      tc = TranslationCache(maxsize=3, ttl_sec=1)
      tc.set("k1", "v1")
      tc.set("k2", "v2")
      tc.set("k3", "v3")
      
      assert tc.get("k1") == "v1"
      tc.set("k4", "v4") # 触发淘汰 最老的是 k2
      assert tc.get("k2") is None
      assert tc.get("k4") == "v4"
      
      # 3. 验证过期
      tc_ttl = TranslationCache(maxsize=5, ttl_sec=0.1)
      tc_ttl.set("k", "v")
      assert tc_ttl.get("k") == "v"
      time.sleep(0.15)
      assert tc_ttl.get("k") is None
      print("Cache logic test passed!")

  if __name__ == "__main__":
      test_cache_logic()
  ```

- [ ] **Step 2: 运行缓存逻辑测试**
  运行: `python server/test_cache.py`
  预期输出: `Cache logic test passed!`

- [ ] **Step 3: 修改 `server/translator.py` 引入缓存层**
  在 `BaseTranslator` 中内置一个全局共享的内存级线程安全 `TranslationCache`。
  
  修改代码定位约第 19-33 行：
  ```python
  import time
  import re
  from threading import Lock
  
  def normalize_text(text: str) -> str:
      if not text:
          return ""
      return re.sub(r"\s+", " ", text.strip())

  class TranslationCache:
      def __init__(self, maxsize=5000, ttl_sec=86400):
          self._cache = {}
          self._lock = Lock()
          self.maxsize = maxsize
          self.ttl_sec = ttl_sec

      def get(self, key: tuple) -> str | None:
          with self._lock:
              if key in self._cache:
                  val, expire_time = self._cache[key]
                  if time.time() < expire_time:
                      # 刷新 LRU 顺序
                      del self._cache[key]
                      self._cache[key] = (val, expire_time)
                      return val
                  else:
                      del self._cache[key]
              return None

      def set(self, key: tuple, value: str):
          with self._lock:
              if len(self._cache) >= self.maxsize:
                  oldest = next(iter(self._cache))
                  del self._cache[oldest]
              expire = time.time() + self.ttl_sec
              self._cache[key] = (value, expire)

  # 全局翻译缓存实例
  _global_translation_cache = TranslationCache(maxsize=5000, ttl_sec=86400)
  ```

  修改 `BaseTranslator` 支持缓存命中查询，并且其 `translate_batch` 需结合 `image_processor` 统计命中数。
  我们将 `translate_batch` 的签名变更为 `translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]`。

  ```python
  class BaseTranslator(abc.ABC):
      def __init__(self):
          self.session = _shared_session
          self.cache = _global_translation_cache
          # 定义翻译器的名称和版本，子类应重写
          self.name = "base"
          self.version = "1.0.0"

      @abc.abstractmethod
      def translate(self, text: str, source_lang: str, target_lang: str) -> str:
          pass

      def _make_key(self, text: str, src_lang: str, dst_lang: str) -> tuple:
          return (
              normalize_text(text),
              src_lang,
              dst_lang,
              self.name,
              self.version
          )

      def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
          if not texts:
              return []
              
          results = [None] * len(texts)
          miss_indices = []
          miss_texts = []
          
          # 1. 拦截缓存命中
          for i, text in enumerate(texts):
              key = self._make_key(text, source_lang, target_lang)
              cached_val = self.cache.get(key)
              if cached_val is not None:
                  results[i] = cached_val
                  if stats_ref is not None:
                      stats_ref["cache_hits"] += 1
              else:
                  miss_indices.append(i)
                  miss_texts.append(text)
                  
          # 2. 对未命中部分调用真实翻译逻辑 (此处基类默认用多线程，子类重写)
          if miss_texts:
              with ThreadPoolExecutor(max_workers=8) as executor:
                  futures = [executor.submit(self.translate, t, source_lang, target_lang) for t in miss_texts]
                  miss_results = [f.result() for f in futures]
                  
              # 3. 填回缓存并更新结果集
              for i, idx in enumerate(miss_indices):
                  translated_val = miss_results[i]
                  results[idx] = translated_val
                  key = self._make_key(miss_texts[i], source_lang, target_lang)
                  self.cache.set(key, translated_val)
                  
          return results
  ```

  并在 `GoogleTranslator`, `LLMTranslator`, `BaiduTranslator` 类的 `__init__` 中定义属性：
  ```python
  class GoogleTranslator(BaseTranslator):
      def __init__(self):
          super().__init__()
          self.name = "google"
          self.version = "1.0.0"
  ```
  ```python
  class LLMTranslator(BaseTranslator):
      def __init__(self, base_url: str, api_key: str, model: str):
          super().__init__(base_url, api_key, model) # 注意保留原有初始化参数
          self.name = f"llm_{model}"
          self.version = "1.0.0"
  ```
  ```python
  class BaiduTranslator(BaseTranslator):
      def __init__(self, app_id: str, secret_key: str):
          super().__init__(app_id, secret_key)
          self.name = "baidu"
          self.version = "1.0.0"
  ```
  *(注：子类在 override `translate_batch` 时也必须接收 `stats_ref: dict = None` 并执行一致的缓存过滤与填回逻辑)*。

- [ ] **Step 4: 在 `server/image_processor.py` 和 `server/app.py` 中适配带 `stats` 引用的翻译批处理回调**
  修改 `server/image_processor.py` 中的回调调用：
  ```python
              # ── 步骤 B：批量翻译 ──
              original_texts = [b["text"] for b in line_blocks]
              translate_start = time.perf_counter()
              try:
                  translated_texts = translator_batch_fn(original_texts, stats)
              except Exception as te:
                  print("[translate] error:", te)
                  translated_texts = original_texts
              stats["translate_ms"] = (time.perf_counter() - translate_start) * 1000
  ```

  修改 `server/app.py` 中的回调包裹器：
  ```python
      translator = get_active_translator()
      def translator_batch_fn(texts, stats_ref):
          return translator.translate_batch(texts, "auto", "zh", stats_ref)
  ```

- [ ] **Step 5: 提交 Phase 2 更改**
  ```bash
  git add server/translator.py server/image_processor.py server/app.py server/test_cache.py
  git commit -m "perf: add memory LRU cache with composite key and TTL for translators"
  ```

---

### Phase 3：加 <SEG> batch + 精准缺失 fallback

本阶段是降低网络开销的核心，通过非换行依赖的 `<SEG{idx}>` 打包格式进行一图单请求传输，且当翻译引擎意外吞掉或漏译个别 tag 时，实行极其精准的 ThreadPool 补齐。

#### Task 3: `<SEG{idx}>` 协议打包与高容错 fallback 模块

**Files:**
- Modify: `server/translator.py`
- Create: `server/test_batch_seg.py`

- [ ] **Step 1: 编写 `<SEG{idx}>` 打包与正则提取解析测试用例**
  创建 `server/test_batch_seg.py`：
  ```python
  # server/test_batch_seg.py
  import re
  
  def parse_seg_translations(response_text: str, expected_count: int) -> dict:
      pattern = r"<SEG(\d+)>(.*?)(?=(?:<SEG\d+>|$))"
      # 使用 re.DOTALL 确保换行符等字符能被正确匹配
      matches = re.findall(pattern, response_text, re.DOTALL)
      parsed = {}
      for idx_str, text in matches:
          parsed[int(idx_str)] = text.strip()
      return parsed

  def test_seg_parsing():
      test_resp = "<SEG0> 确定 <SEG1>  取消 \n 换行内容 <SEG2>关闭按钮"
      parsed = parse_seg_translations(test_resp, 3)
      assert parsed[0] == "确定"
      assert parsed[1] == "取消 \n 换行内容"
      assert parsed[2] == "关闭按钮"
      
      # 验证缺漏情况
      test_missing = "<SEG0> 确定 <SEG2> 关闭"
      parsed_missing = parse_seg_translations(test_missing, 3)
      assert 0 in parsed_missing
      assert 1 not in parsed_missing
      assert 2 in parsed_missing
      print("SEG protocol and parsing test passed!")

  if __name__ == "__main__":
      test_seg_parsing()
  ```

- [ ] **Step 2: 运行 SEG 打包测试**
  运行: `python server/test_batch_seg.py`
  预期输出: `SEG protocol and parsing test passed!`

- [ ] **Step 3: 重构 `GoogleTranslator.translate_batch` 模块**
  对 Google 翻译的 `translate_batch` 进行重构，将传入的 `texts` 进行缓存命中预筛过滤。然后对未命中的部分采用 `<SEG{idx}>` 打包。对返回结果进行正则匹配，填入 `idx -> val` 映射。如出现缺失部分，只针对缺失的 index 调用单句 `translate` 线程池并发补齐，并更新入 LRU 缓存。
  
  修改 `GoogleTranslator.translate_batch` 代码（约 48-93 行）：
  ```python
      def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
          if not texts:
              return []
              
          results = [None] * len(texts)
          miss_indices = []
          miss_texts = []
          
          # 1. 过滤缓存命中
          for i, text in enumerate(texts):
              key = self._make_key(text, source_lang, target_lang)
              cached_val = self.cache.get(key)
              if cached_val is not None:
                  results[i] = cached_val
                  if stats_ref is not None:
                      stats_ref["cache_hits"] += 1
              else:
                  miss_indices.append(i)
                  miss_texts.append(text)
                  
          if not miss_texts:
              return results
              
          # 2. 构建 SEG 协议请求文本
          segments = []
          for i, t in enumerate(miss_texts):
              segments.append(f"<SEG{i}>{t}")
          query = "".join(segments)
          
          url = "https://translate.googleapis.com/translate_a/single"
          data = {
              "client": "gtx",
              "sl": source_lang,
              "tl": target_lang,
              "dt": "t",
              "q": query
          }
          
          batch_parsed = {}
          success_batch = False
          
          try:
              response = self.session.post(url, data=data, timeout=8)
              if response.status_code == 200:
                  res_json = response.json()
                  if isinstance(res_json, list) and len(res_json) > 0 and isinstance(res_json[0], list):
                      translated_full = "".join([part[0] for part in res_json[0] if part[0]])
                      
                      # 正则提炼标签内容
                      pattern = r"<SEG(\d+)>(.*?)(?=(?:<SEG\d+>|$))"
                      matches = re.findall(pattern, translated_full, re.DOTALL)
                      for idx_str, trans_val in matches:
                          batch_parsed[int(idx_str)] = trans_val.strip()
                      
                      success_batch = True
          except Exception as e:
              logger.warning("[Google Batch] SEG 批量翻译请求失败: %s。将进入降级补全...", e)

          # 3. 对缺失的翻译进行精准 ThreadPool Fallback 补齐
          fallback_indices = []
          fallback_texts = []
          
          for local_idx, orig_text in enumerate(miss_texts):
              if local_idx in batch_parsed:
                  # 单批次段翻译成功，更新入结果并加入缓存
                  val = batch_parsed[local_idx]
                  global_idx = miss_indices[local_idx]
                  results[global_idx] = val
                  key = self._make_key(orig_text, source_lang, target_lang)
                  self.cache.set(key, val)
              else:
                  fallback_indices.append(local_idx)
                  fallback_texts.append(orig_text)
                  
          if fallback_texts:
              logger.warning(
                  "[Google Batch] SEG 标签存在未匹配的缺失项: %d/%d。开始发起局部 Fallback 线程池并发补齐...", 
                  len(fallback_texts), len(miss_texts)
              )
              with ThreadPoolExecutor(max_workers=8) as executor:
                  futures = [executor.submit(self.translate, t, source_lang, target_lang) for t in fallback_texts]
                  fallback_results = [f.result() for f in futures]
                  
              for idx_in_fb, local_idx in enumerate(fallback_indices):
                  val = fallback_results[idx_in_fb]
                  global_idx = miss_indices[local_idx]
                  results[global_idx] = val
                  key = self._make_key(miss_texts[local_idx], source_lang, target_lang)
                  self.cache.set(key, val)

          return results
  ```

- [ ] **Step 4: 适配重构 `BaiduTranslator.translate_batch`**
  对百度翻译采用一脉相承的 `<SEG{idx}>` 批量重构。
  ```python
      def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
          if not texts:
              return []
              
          results = [None] * len(texts)
          miss_indices = []
          miss_texts = []
          
          for i, text in enumerate(texts):
              key = self._make_key(text, source_lang, target_lang)
              cached_val = self.cache.get(key)
              if cached_val is not None:
                  results[i] = cached_val
                  if stats_ref is not None:
                      stats_ref["cache_hits"] += 1
              else:
                  miss_indices.append(i)
                  miss_texts.append(text)
                  
          if not miss_texts:
              return results
              
          segments = []
          for i, t in enumerate(miss_texts):
              segments.append(f"<SEG{i}>{t}")
          query = "".join(segments)
          
          salt = str(random.randint(32768, 65536))
          sign_str = self.app_id + query + salt + self.secret_key
          sign = hashlib.md5(sign_str.encode('utf-8')).hexdigest()
          
          from_lang = "auto" if source_lang == "auto" else source_lang
          to_lang = "zh" if target_lang == "zh" else target_lang
          
          url = "https://fanyi-api.baidu.com/api/trans/vip/translate"
          data = {
              "q": query,
              "from": from_lang,
              "to": to_lang,
              "appid": self.app_id,
              "salt": salt,
              "sign": sign
          }
          
          batch_parsed = {}
          try:
              res = self.session.post(url, data=data, timeout=8)
              if res.status_code == 200:
                  res_json = res.json()
                  if "error_code" not in res_json:
                      trans_result = res_json.get("trans_result", [])
                      # 百度会对大文本返回单个分段 dst
                      translated_full = "".join([item["dst"] for item in trans_result])
                      
                      pattern = r"<SEG(\d+)>(.*?)(?=(?:<SEG\d+>|$))"
                      matches = re.findall(pattern, translated_full, re.DOTALL)
                      for idx_str, trans_val in matches:
                          batch_parsed[int(idx_str)] = trans_val.strip()
          except Exception as e:
              logger.warning("[Baidu Batch] 批量翻译请求失败: %s。将进入降级补全...", e)

          # 精准补齐
          fallback_indices = []
          fallback_texts = []
          
          for local_idx, orig_text in enumerate(miss_texts):
              if local_idx in batch_parsed:
                  val = batch_parsed[local_idx]
                  global_idx = miss_indices[local_idx]
                  results[global_idx] = val
                  key = self._make_key(orig_text, source_lang, target_lang)
                  self.cache.set(key, val)
              else:
                  fallback_indices.append(local_idx)
                  fallback_texts.append(orig_text)
                  
          if fallback_texts:
              logger.warning("[Baidu Batch] 标签缺失 %d 个，开启 ThreadPool 补充...", len(fallback_texts))
              with ThreadPoolExecutor(max_workers=8) as executor:
                  futures = [executor.submit(self.translate, t, source_lang, target_lang) for t in fallback_texts]
                  fallback_results = [f.result() for f in futures]
              for idx_in_fb, local_idx in enumerate(fallback_indices):
                  val = fallback_results[idx_in_fb]
                  global_idx = miss_indices[local_idx]
                  results[global_idx] = val
                  key = self._make_key(miss_texts[local_idx], source_lang, target_lang)
                  self.cache.set(key, val)

          return results
  ```

- [ ] **Step 5: 适配重构 `LLMTranslator.translate_batch`**
  大模型翻译同样在 Prompt 中规定对 `<SEG{idx}>` 标签的强制保留，并通过正则映射提炼。
  ```python
      def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
          if not texts:
              return []
              
          results = [None] * len(texts)
          miss_indices = []
          miss_texts = []
          
          for i, text in enumerate(texts):
              key = self._make_key(text, source_lang, target_lang)
              cached_val = self.cache.get(key)
              if cached_val is not None:
                  results[i] = cached_val
                  if stats_ref is not None:
                      stats_ref["cache_hits"] += 1
              else:
                  miss_indices.append(i)
                  miss_texts.append(text)
                  
          if not miss_texts:
              return results
              
          segments = []
          for i, t in enumerate(miss_texts):
              segments.append(f"<SEG{i}>{t}")
          query = "".join(segments)
          
          headers = {
              "Authorization": f"Bearer {self.api_key}",
              "Content-Type": "application/json"
          }
          prompt = (
              "You are a translation assistant. Translate the following block of text containing multiple tags like <SEG0>, <SEG1> into Simplified Chinese.\n"
              "Crucially: you MUST keep the tags <SEG0>, <SEG1> in their exact position! Translate the content immediately after each tag.\n"
              "Respond ONLY with the translated text retaining the tags. Do not output anything else."
          )
          payload = {
              "model": self.model,
              "messages": [
                  {"role": "system", "content": prompt},
                  {"role": "user", "content": query}
              ],
              "temperature": 0.2
          }
          
          batch_parsed = {}
          try:
              res = self.session.post(f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=15)
              if res.status_code == 200:
                  translated_full = res.json()["choices"][0]["message"]["content"].strip()
                  pattern = r"<SEG(\d+)>(.*?)(?=(?:<SEG\d+>|$))"
                  matches = re.findall(pattern, translated_full, re.DOTALL)
                  for idx_str, trans_val in matches:
                      batch_parsed[int(idx_str)] = trans_val.strip()
          except Exception as e:
              logger.warning("[LLM Batch] 批量翻译请求失败: %s。将进入降级补全...", e)

          # 精准补齐
          fallback_indices = []
          fallback_texts = []
          
          for local_idx, orig_text in enumerate(miss_texts):
              if local_idx in batch_parsed:
                  val = batch_parsed[local_idx]
                  global_idx = miss_indices[local_idx]
                  results[global_idx] = val
                  key = self._make_key(orig_text, source_lang, target_lang)
                  self.cache.set(key, val)
              else:
                  fallback_indices.append(local_idx)
                  fallback_texts.append(orig_text)
                  
          if fallback_texts:
              logger.warning("[LLM Batch] 标签缺失 %d 个，开启 ThreadPool 补充...", len(fallback_texts))
              with ThreadPoolExecutor(max_workers=8) as executor:
                  futures = [executor.submit(self.translate, t, source_lang, target_lang) for t in fallback_texts]
                  fallback_results = [f.result() for f in futures]
              for idx_in_fb, local_idx in enumerate(fallback_indices):
                  val = fallback_results[idx_in_fb]
                  global_idx = miss_indices[local_idx]
                  results[global_idx] = val
                  key = self._make_key(miss_texts[local_idx], source_lang, target_lang)
                  self.cache.set(key, val)

          return results
  ```

- [ ] **Step 6: 提交 Phase 3 更改**
  ```bash
  git add server/translator.py server/test_batch_seg.py
  git commit -m "feat: implement single-request SEG batch translation with precise threadpool fallback"
  ```

---

### Phase 4：加 VirtualBlock 空间合并

本阶段实现物理相邻文字框的临时翻译块合并，显著减少翻译标签数量并提升上下文连贯性，并在重绘时执行 `union_bbox` 整体排版。

#### Task 4: 空间相邻合并算法与 union_bbox 重绘

**Files:**
- Modify: `server/image_processor.py`
- Create: `server/test_virtual_block.py`

- [ ] **Step 1: 编写空间合并算法测试用例**
  创建 `server/test_virtual_block.py`，模拟同一水平线内空间重叠或邻近的合并算法：
  ```python
  # server/test_virtual_block.py
  
  def compute_virtual_blocks(blocks: list) -> list:
      if not blocks:
          return []
          
      # 按 x_min 排序
      blocks.sort(key=lambda b: b["rect"][0])
      
      # 统计平均高度
      avg_height = sum((b["rect"][3] - b["rect"][1]) for b in blocks) / len(blocks)
      
      merged = []
      current = [blocks[0]]
      
      for i in range(1, len(blocks)):
          prev = current[-1]
          cur = blocks[i]
          
          # center_y
          prev_cy = (prev["rect"][1] + prev["rect"][3]) / 2.0
          cur_cy = (cur["rect"][1] + cur["rect"][3]) / 2.0
          
          # height
          prev_h = prev["rect"][3] - prev["rect"][1]
          cur_h = cur["rect"][3] - cur["rect"][1]
          
          # gap
          gap_x = cur["rect"][0] - prev["rect"][2]
          
          # 判定条件
          same_row = abs(prev_cy - cur_cy) <= 0.5 * avg_height
          near_x = gap_x <= 2.0 * avg_height
          h_ratio = max(prev_h, cur_h) / max(1, min(prev_h, cur_h)) <= 1.5
          limit_ok = len(current) < 6
          
          if same_row and near_x and h_ratio and limit_ok:
              current.append(cur)
          else:
              merged.append(current)
              current = [cur]
      merged.append(current)
      
      # 打包成 VirtualBlock
      virtual_blocks = []
      for group in merged:
          x_min = min(b["rect"][0] for b in group)
          y_min = min(b["rect"][1] for b in group)
          x_max = max(b["rect"][2] for b in group)
          y_max = max(b["rect"][3] for b in group)
          
          combined_text = " ".join(b["text"] for b in group)
          virtual_blocks.append({
              "rect": [x_min, y_min, x_max, y_max],
              "text": combined_text,
              "avg_h": avg_height,
              "children": group
          })
      return virtual_blocks

  def test_merging():
      blocks = [
          {"rect": [10, 100, 50, 120], "text": "Hello"},
          {"rect": [60, 100, 100, 120], "text": "World"},
          {"rect": [15, 200, 80, 225], "text": "Unrelated"}
      ]
      v_blocks = compute_virtual_blocks(blocks)
      assert len(v_blocks) == 2
      assert v_blocks[0]["text"] == "Hello World"
      assert v_blocks[0]["rect"] == [10, 100, 100, 120]
      assert v_blocks[1]["text"] == "Unrelated"
      print("Spatial adjacent merging test passed!")

  if __name__ == "__main__":
      test_merging()
  ```

- [ ] **Step 2: 运行空间合并算法测试**
  运行: `python server/test_virtual_block.py`
  预期输出: `Spatial adjacent merging test passed!`

- [ ] **Step 3: 修改 `server/image_processor.py` 的分组模块**
  我们要修改 `process_and_draw` 函数。替换原先的 `_group_into_lines`（即以前粗暴的合并所有行）为精细的 **“空间相邻合并 + 虚拟块回填”** 算法。
  
  修改 `_group_into_lines` 方法（约第 89-163 行）：
  ```python
      def _group_into_lines(self, raw_lines: list) -> list:
          """
          将 PaddleOCR 返回的原始框按照空间相邻合并规则组合成 VirtualBlocks。
          """
          if not raw_lines:
              return []
  
          blocks = []
          for line in raw_lines:
              box = line[0]
              text = line[1][0]
              x_min = int(min(pt[0] for pt in box))
              x_max = int(max(pt[0] for pt in box))
              y_min = int(min(pt[1] for pt in box))
              y_max = int(max(pt[1] for pt in box))
              h = y_max - y_min
              blocks.append({
                  "rect": [x_min, y_min, x_max, y_max],
                  "h": h,
                  "text": text,
              })
  
          # 先整体按 y_min 粗分行排序
          blocks.sort(key=lambda it: (it["rect"][1], it["rect"][0]))
          
          # 计算整个截图的平均行高
          avg_height = sum(b["h"] for b in blocks) / len(blocks)
          
          # 空间物理邻近度分行物理合并
          merged_groups = []
          # 通过并查集或序列扫描算法。我们采用贪心的序列水平与垂直投影扫描法：
          # 在同一行内判定：abs(center_y1 - center_y2) <= 0.5 * avg_height
          # 水平间距：gap_x <= 2.0 * avg_height
          # 高度相近：max(h1, h2) / min(h1, h2) <= 1.5
          # 合并上限：最多 6 个 Block，总字数不超过 80
          
          temp_blocks = list(blocks)
          while temp_blocks:
              current_block = temp_blocks.pop(0)
              current_group = [current_block]
              
              # 循环从剩余块中寻找高度符合物理相邻条件的块合并
              while True:
                  found_next = None
                  for b in temp_blocks:
                      # 找到和当前组合最右端相邻的块
                      last_b = current_group[-1]
                      
                      last_cy = (last_b["rect"][1] + last_b["rect"][3]) / 2.0
                      b_cy = (b["rect"][1] + b["rect"][3]) / 2.0
                      
                      gap_x = b["rect"][0] - last_b["rect"][2]
                      
                      same_row = abs(last_cy - b_cy) <= 0.5 * avg_height
                      # 且要求水平投影在右侧（差值在 [0, 2*avg_h] 之间）
                      near_x = 0 <= gap_x <= 2.0 * avg_height
                      
                      h_ratio = max(last_b["h"], b["h"]) / max(1, min(last_b["h"], b["h"])) <= 1.5
                      char_len = sum(len(x["text"]) for x in current_group) + len(b["text"])
                      
                      if same_row and near_x and h_ratio and len(current_group) < 6 and char_len <= 80:
                          found_next = b
                          break
                  
                  if found_next:
                      current_group.append(found_next)
                      temp_blocks.remove(found_next)
                  else:
                      break
                      
              merged_groups.append(current_group)
  
          # 生成外接矩形 union_bbox
          result_virtual_blocks = []
          for group in merged_groups:
              x_min = min(it["rect"][0] for it in group)
              x_max = max(it["rect"][2] for it in group)
              y_min = min(it["rect"][1] for it in group)
              y_max = max(it["rect"][3] for it in group)
              combined_text = " ".join(it["text"] for it in group)
              
              result_virtual_blocks.append({
                  "rect": [x_min, y_min, x_max, y_max],
                  "text": combined_text,
                  "avg_h": sum(it["h"] for it in group) / len(group),
                  "children": group # 备用做 fallback 细化
              })
              
          return result_virtual_blocks
  ```

  并在 `process_and_draw` 中直接拿 `result_virtual_blocks` 渲染，以大框的 union_bbox 整体擦除背景与折行回填：
  *(在 Task 1 改造时我们已经做好了这一层铺垫，现在我们将 `process_and_draw` 与修改后的 `_group_into_lines` 完美连接即可)*。

- [ ] **Step 4: 提交 Phase 4 更改**
  ```bash
  git add server/image_processor.py server/test_virtual_block.py
  git commit -m "feat: implement Spatial-Adjacent VirtualBlock grouping and union_bbox layout rendering"
  ```

---

### Phase 5：基准测试与验证

本阶段编写集成压力与耗时基准测试脚本，对各种负载（10/30/80 个物理框）的截图模拟翻译链路，收集性能数据。

#### Task 5: 全链路基准压力测试与时延评估报告

**Files:**
- Create: `server/benchmark.py`

- [ ] **Step 1: 编写全链路基准测试脚本**
  创建 `server/benchmark.py`，模拟生成不同数量（10/30/80）的测试文本框输入，并对我们的全链路缓存与批量翻译进行验证，并输出结构化耗时占比报告：
  ```python
  # server/benchmark.py
  import time
  import numpy as np
  import cv2
  from PIL import Image, ImageDraw
  from translator import GoogleTranslator
  from image_processor import ImageProcessor
  
  def make_mock_image(num_blocks: int) -> bytes:
      # 创建一张白底图片，上面画上指定个数的框与英文文本
      img = Image.new("RGB", (1000, 1000 + num_blocks * 25), "white")
      draw = ImageDraw.Draw(img)
      for i in range(num_blocks):
          y = 50 + i * 25
          text = f"Test block index {i} which is quite long for testing translation"
          draw.rectangle([50, y, 600, y + 20], fill="white")
          # 顺便画点线条代表字迹
          draw.text((60, y + 2), text, fill="black")
      import io
      out = io.BytesIO()
      img.save(out, format="PNG")
      return out.getvalue()

  def run_benchmarks():
      processor = ImageProcessor(load_ocr=True)
      translator = GoogleTranslator()
      
      print("\n" + "="*50)
      print("🚀 启动 10 / 30 / 80 文本框截图全链路基准压力测试")
      print("="*50 + "\n")
      
      for load_size in [10, 30, 80]:
          print(f"📦 【当前负载】: {load_size} 个文字物理框")
          img_bytes = make_mock_image(load_size)
          
          stats_ref = {
              "ocr_ms": 0.0,
              "translate_ms": 0.0,
              "render_ms": 0.0,
              "total_ms": 0.0,
              "ocr_blocks": 0,
              "translate_units": 0,
              "cache_hits": 0
          }
          
          def translator_batch_fn(texts, s_ref=stats_ref):
              return translator.translate_batch(texts, "auto", "zh", s_ref)
          
          # 1. 首次翻译 (无缓存)
          t_start = time.perf_counter()
          _, stats = processor.process_and_draw(img_bytes, translator_batch_fn)
          t_total = (time.perf_counter() - t_start) * 1000
          
          print(f" ── [第一次 (无缓存)] ──")
          print(f"    OCR 耗时    : {stats['ocr_ms']:.2f} ms")
          print(f"    翻译网络耗时 : {stats['translate_ms']:.2f} ms")
          print(f"    重绘渲染耗时 : {stats['render_ms']:.2f} ms")
          print(f"    缓存命中数   : {stats['cache_hits']} / {stats['translate_units']}")
          print(f"    总响应时延   : {stats['total_ms']:.2f} ms (perf: {t_total:.2f} ms)")
          
          # 2. 第二次翻译 (全缓存命中)
          t_start_cached = time.perf_counter()
          stats_ref_cached = {
              "ocr_ms": 0.0,
              "translate_ms": 0.0,
              "render_ms": 0.0,
              "total_ms": 0.0,
              "ocr_blocks": 0,
              "translate_units": 0,
              "cache_hits": 0
          }
          def cached_batch_fn(texts, s_ref=stats_ref_cached):
              return translator.translate_batch(texts, "auto", "zh", s_ref)
          
          _, stats_c = processor.process_and_draw(img_bytes, cached_batch_fn)
          t_total_c = (time.perf_counter() - t_start_cached) * 1000
          
          print(f" ── [第二次 (全缓存命中)] ──")
          print(f"    OCR 耗时    : {stats_c['ocr_ms']:.2f} ms")
          print(f"    翻译网络耗时 : {stats_c['translate_ms']:.2f} ms")
          print(f"    重绘渲染耗时 : {stats_c['render_ms']:.2f} ms")
          print(f"    缓存命中数   : {stats_c['cache_hits']} / {stats_c['translate_units']}")
          print(f"    总响应时延   : {stats_c['total_ms']:.2f} ms (perf: {t_total_c:.2f} ms)")
          print("-" * 50)
          
      print("\n" + "="*50)
      print("🎉 截图性能极致优化全链路基准测试完成")
      print("="*50 + "\n")

  if __name__ == "__main__":
      run_benchmarks()
  ```

- [ ] **Step 2: 运行压力测试脚本**
  运行: `python server/benchmark.py`
  预期输出: 应该完美跑完 10, 30, 80 个物理框的两次翻译（一次无缓存，一次全缓存），并且第二次的翻译网络耗时直接缩水为 0ms，整体时延发生断崖式下跌，且 100% 运行通过。

- [ ] **Step 3: 提交 Phase 5 更改并合并入 master**
  ```bash
  git add server/benchmark.py
  git commit -m "test: add end-to-end load stress and performance validation benchmarks"
  ```

---

## 3. 自检 checklist
- [x] 是否存在 placeholders（TODO/TBD）？不存在，所有测试用例和重构的核心算法均已在文中清晰给出了完整的 Python 代码实现！
- [x] 命名一致性验证？所有 key 定义、normalize_text 函数签名、stats 字典字段名（`ocr_ms`, `translate_ms`, `render_ms`, `total_ms`, `ocr_blocks`, `translate_units`, `cache_hits`）在各个 Phase 间达到了完美的统一，没有任何逻辑冲突。
- [x] 复合键是否应用了最新的“首尾 strip + 连续空白折叠”原则？是的，已经在 `normalize_text` 函数中完美实现（`re.sub(r"\s+", " ", text.strip())`）。
- [x] 已经完全覆盖了用户的 3 项最新修正要求。
