import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
import os
import io
import threading
import time


class ImageProcessor:
    def __init__(self, load_ocr: bool = True):
        self._ocr_lock = threading.Lock()
        self._font_cache = {}
        self._font_path = None
        if load_ocr:
            self._ensure_ocr()
        else:
            self.ocr = None

    def _ensure_ocr(self):
        with self._ocr_lock:
            if self.ocr is None:
                from paddleocr import PaddleOCR
                self.ocr = PaddleOCR(
                    lang="ch",
                    enable_mkldnn=False,
                    det_db_box_thresh=0.3,
                    det_db_thresh=0.2,
                    det_db_unclip_ratio=1.6
                )

    # ──────────────────────────────────────────────
    # 1. 字体加载
    # ──────────────────────────────────────────────
    def _load_font(self, size: int) -> ImageFont.FreeTypeFont:
        if size in self._font_cache:
            return self._font_cache[size]

        if self._font_path is not None:
            active = self._font_path
        else:
            user_font_dir = os.path.expanduser("~/.screenshot-translator")
            os.makedirs(user_font_dir, exist_ok=True)
            user_font_path = os.path.join(user_font_dir, "wqy-microhei.ttc")

            font_paths = [
                user_font_path,
                "C:\\Windows\\Fonts\\msyh.ttc",        # 微软雅黑
                "C:\\Windows\\Fonts\\msyhbd.ttc",       # 微软雅黑 Bold
                "C:\\Windows\\Fonts\\simhei.ttf",       # 黑体
                "C:\\Windows\\Fonts\\simsun.ttc",       # 宋体
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
                "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
                "/usr/share/fonts/truetype/arphic/uming.ttc",
            ]
            active = next((p for p in font_paths if os.path.exists(p)), None)

            if not active:
                try:
                    print("[FontManager] 未找到中文字体，正在从 CDN 下载文泉驿微米黑…")
                    import urllib.request
                    url = "https://cdn.jsdelivr.net/gh/anthonyfok/fonts-wqy-microhei@master/wqy-microhei.ttc"
                    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
                    with urllib.request.urlopen(req, timeout=15) as r, open(user_font_path, "wb") as f:
                        f.write(r.read())
                    if os.path.getsize(user_font_path) > 1_000_000:
                        active = user_font_path
                        print("[FontManager] 字体下载成功:", active)
                except Exception as fe:
                    print("[FontManager] 字体下载失败:", fe)

            if active:
                self._font_path = active

        if active:
            font = ImageFont.truetype(active, size)
        else:
            font = ImageFont.load_default()
            
        self._font_cache[size] = font
        return font

    # ──────────────────────────────────────────────
    # 2. OCR box 分行合并算法
    # ──────────────────────────────────────────────
    def _group_into_lines(self, raw_lines: list) -> list:
        """
        将 PaddleOCR 返回的原始 box 列表根据空间几何相邻性合并为 VirtualBlock。
        避免跨栏、跨行非相邻的文本块被强行连接，保障排版完整性。
        """
        if not raw_lines:
            return []

        # 1. 结构化物理框
        items = []
        for line in raw_lines:
            box = line[0]   # [[x1,y1],[x2,y2],[x3,y3],[x4,y4]]
            text = line[1][0].strip()
            confidence = line[1][1]
            
            xs = [pt[0] for pt in box]
            ys = [pt[1] for pt in box]
            x_min, y_min, x_max, y_max = min(xs), min(ys), max(xs), max(ys)
            
            w = x_max - x_min
            h = y_max - y_min
            cy = y_min + h / 2.0
            
            items.append({
                "rect": [x_min, y_min, x_max, y_max],
                "text": text,
                "w": w,
                "h": h,
                "cy": cy,
                "confidence": confidence
            })
            
        # 按 Y 坐标排序，自上而下开始行合并
        items.sort(key=lambda b: b["rect"][1])
        
        # 2. 合并同行且在水平空间上绝对相邻的物理块
        virtual_blocks = []
        
        while items:
            current = items.pop(0)
            merged_group = [current]
            
            i = 0
            while i < len(items):
                candidate = items[i]
                last = merged_group[-1]
                avg_h = (last["h"] + candidate["h"]) / 2.0
                
                # 同行判断: 中心 Y 距离小于平均高度的 0.6 倍
                same_line = abs(last["cy"] - candidate["cy"]) <= 0.6 * avg_h
                
                # 水平间距: 间距小于平均高度的 2.2 倍，且不能为负数（重叠/错位）
                gap_x = candidate["rect"][0] - last["rect"][2]
                horizontal_near = 0 <= gap_x <= 2.2 * avg_h
                
                # 高度相近: 两个块高度比例小于 1.5 倍
                height_similar = (max(last["h"], candidate["h"]) / max(min(last["h"], candidate["h"]), 0.001)) <= 1.5
                
                # 长度约束: 合并后的字符数限制 <= 80，合并块数 <= 6
                merged_len = sum(len(b["text"]) for b in merged_group) + len(candidate["text"])
                count_ok = len(merged_group) < 6
                
                if same_line and horizontal_near and height_similar and merged_len <= 80 and count_ok:
                    merged_group.append(candidate)
                    items.pop(i)
                else:
                    i += 1
                    
            # 聚合当前合并组
            all_xs = []
            all_ys = []
            texts_to_join = []
            
            for b in merged_group:
                r = b["rect"]
                all_xs.extend([r[0], r[2]])
                all_ys.extend([r[1], r[3]])
                texts_to_join.append(b["text"])
                
            union_x1 = min(all_xs)
            union_y1 = min(all_ys)
            union_x2 = max(all_xs)
            union_y2 = max(all_ys)
            
            union_text = " ".join(texts_to_join)
            avg_height = sum(b["h"] for b in merged_group) / len(merged_group)
            
            virtual_blocks.append({
                "rect": [int(union_x1), int(union_y1), int(union_x2), int(union_y2)],
                "text": union_text,
                "avg_h": avg_height,
            })
            
        return virtual_blocks

    # ──────────────────────────────────────────────
    # 3. 背景颜色采样
    # ──────────────────────────────────────────────
    def _sample_bg(self, img_cv: np.ndarray, x1: int, y1: int, x2: int, y2: int) -> tuple:
        """
        在 box 外扩 4px 的环形区域采样像素，用中位数颜色作为背景色（BGR）。
        使用 NumPy 向量化操作加速。
        """
        h, w = img_cv.shape[:2]
        pad = 4
        
        ry1 = max(0, y1 - pad)
        ry2 = min(h, y2 + pad)
        rx1 = max(0, x1 - pad)
        rx2 = min(w, x2 + pad)
        
        if ry1 >= ry2 or rx1 >= rx2:
            return (255, 255, 255)
            
        region = img_cv[ry1:ry2, rx1:rx2]
        
        iy1 = y1 - ry1
        iy2 = y2 - ry1
        ix1 = x1 - rx1
        ix2 = x2 - rx1
        
        mask = np.ones((region.shape[0], region.shape[1]), dtype=bool)
        mask[max(0, iy1):min(region.shape[0], iy2), max(0, ix1):min(region.shape[1], ix2)] = False
        
        pixels = region[mask]
        if pixels.size == 0:
            return (255, 255, 255)
            
        med = np.median(pixels, axis=0)
        return (int(med[0]), int(med[1]), int(med[2]))

    def _bg_to_rgb(self, bg_bgr: tuple) -> tuple:
        return (bg_bgr[2], bg_bgr[1], bg_bgr[0])

    def _choose_text_color(self, bg_bgr: tuple) -> tuple:
        """
        根据背景亮度决定文字颜色（亮背景→深字，暗背景→白字）。
        返回 RGB。
        """
        lum = 0.299 * bg_bgr[2] + 0.587 * bg_bgr[1] + 0.114 * bg_bgr[0]
        if lum > 140:
            return (20, 20, 20)     # 深色字
        else:
            return (240, 240, 240)  # 浅色字

    # ──────────────────────────────────────────────
    # 4. Layout engine：动态字号 + 换行
    # ──────────────────────────────────────────────
    def _layout_text(self, draw: ImageDraw.ImageDraw, text: str,
                     box_w: int, box_h: int, avg_h: float) -> tuple:
        """
        给定 box 宽高和平均行高，自动计算最佳字号 + 分行。
        返回 (lines: list[str], font: ImageFont, line_gap: int, actual_h: int)
        """
        # 初始字号取原文行高的 85%（留一点内边距）
        font_size = max(11, int(avg_h * 0.85))
        MIN_SIZE = 10

        for attempt_size in range(font_size, MIN_SIZE - 1, -1):
            font = self._load_font(attempt_size)
            lines = self._wrap_text(draw, text, font, box_w - 4)  # 左右各留 2px padding
            line_gap = int(attempt_size * 1.2)
            total_h = len(lines) * line_gap
            if total_h <= box_h or attempt_size <= MIN_SIZE:
                return lines, font, line_gap, total_h

        # 兜底：用最小字号强行折行
        font = self._load_font(MIN_SIZE)
        lines = self._wrap_text(draw, text, font, box_w - 4)
        line_gap = int(MIN_SIZE * 1.2)
        return lines, font, line_gap, len(lines) * line_gap

    def _wrap_text(self, draw: ImageDraw.ImageDraw, text: str,
                   font: ImageFont.FreeTypeFont, max_w: int) -> list:
        """
        中文字符逐字换行；英文按空格分词换行。
        """
        lines = []
        current = ""
        for ch in text:
            test = current + ch
            try:
                w = draw.textlength(test, font=font)
            except Exception:
                w = len(test) * font.size  # 兜底估算
            if w <= max_w:
                current = test
            else:
                if current:
                    lines.append(current)
                current = ch
        if current:
            lines.append(current)
        return lines if lines else [text]

    # ──────────────────────────────────────────────
    # 5. 主流程
    # ──────────────────────────────────────────────
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
                translated_texts = translator_batch_fn(original_texts, stats)
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

