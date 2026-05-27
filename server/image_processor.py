import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
import os
import io
import threading


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
                    ir_optim=False,
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
        将 PaddleOCR 返回的原始 box 列表按行高 + 行间距合并成逻辑行组。
        每个逻辑行组包含若干个在同一行内的 box，最终合并成一个 LineBlock。

        返回值: list of LineBlock dict:
            {
                "rect": [x_min, y_min, x_max, y_max],   # 整行外围矩形
                "text": "合并后的原文",
                "avg_h": 平均行高（float），用于推算字号
            }
        """
        if not raw_lines:
            return []

        # 解析每个 box 为结构体
        items = []
        for line in raw_lines:
            box = line[0]   # [[x1,y1],[x2,y2],[x3,y3],[x4,y4]]
            text = line[1][0]
            x_min = int(min(pt[0] for pt in box))
            x_max = int(max(pt[0] for pt in box))
            y_min = int(min(pt[1] for pt in box))
            y_max = int(max(pt[1] for pt in box))
            h = y_max - y_min
            items.append({
                "x_min": x_min, "x_max": x_max,
                "y_min": y_min, "y_max": y_max,
                "h": h,
                "text": text,
            })

        # 按 y_min 排序（从上到下）
        items.sort(key=lambda it: it["y_min"])

        groups = []
        current_group = [items[0]]

        for i in range(1, len(items)):
            prev = current_group[-1]
            cur  = items[i]
            avg_h = sum(x["h"] for x in current_group) / len(current_group)

            # 两行中心 y 之差
            prev_cy = (prev["y_min"] + prev["y_max"]) / 2
            cur_cy  = (cur["y_min"]  + cur["y_max"])  / 2
            dy = abs(cur_cy - prev_cy)

            # 同一行判断：两行中心 y 之差 < 0.8 * 平均行高
            # 即：垂直偏移不超过 80% 行高，认为是同一行
            if dy < avg_h * 0.8:
                current_group.append(cur)
            else:
                groups.append(current_group)
                current_group = [cur]

        groups.append(current_group)

        # 将每个 group 内的 box 按 x_min 排序，拼接文本，计算外围矩形
        result = []
        for group in groups:
            group.sort(key=lambda it: it["x_min"])
            merged_text = " ".join(it["text"] for it in group)
            x_min = min(it["x_min"] for it in group)
            x_max = max(it["x_max"] for it in group)
            y_min = min(it["y_min"] for it in group)
            y_max = max(it["y_max"] for it in group)
            avg_h = sum(it["h"] for it in group) / len(group)
            result.append({
                "rect": [x_min, y_min, x_max, y_max],
                "text": merged_text,
                "avg_h": avg_h,
            })

        return result

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
    def process_and_draw(self, img_bytes: bytes, translator_batch_fn) -> bytes:
        try:
            nparr = np.frombuffer(img_bytes, np.uint8)
            img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
            if img_cv is None:
                return img_bytes

            self._ensure_ocr()
            ocr_result = self.ocr.ocr(img_cv, cls=True)
            if not ocr_result or not ocr_result[0]:
                return img_bytes  # 无文字，返回原图

            raw_lines = ocr_result[0]

            # ── 步骤 A：按行合并 OCR box ──
            line_blocks = self._group_into_lines(raw_lines)

            # ── 步骤 B：批量翻译（每逻辑行一条） ──
            original_texts = [b["text"] for b in line_blocks]
            try:
                translated_texts = translator_batch_fn(original_texts)
            except Exception as te:
                print("[translate] error:", te)
                translated_texts = original_texts

            if len(translated_texts) != len(original_texts):
                translated_texts = original_texts

            print(f"[Layout] {len(line_blocks)} line blocks to render")

            # ── 步骤 C：转 PIL、逐行绘制 ──
            pil_img = Image.fromarray(cv2.cvtColor(img_cv, cv2.COLOR_BGR2RGB))
            draw = ImageDraw.Draw(pil_img)

            for block, trans_text in zip(line_blocks, translated_texts):
                x1, y1, x2, y2 = block["rect"]
                avg_h = block["avg_h"]
                box_w = x2 - x1
                box_h = y2 - y1

                if box_w <= 0 or box_h <= 0:
                    continue

                # 采样背景色
                bg_bgr = self._sample_bg(img_cv, x1, y1, x2, y2)
                bg_rgb = self._bg_to_rgb(bg_bgr)
                text_rgb = self._choose_text_color(bg_bgr)

                # 背景擦除：外扩 3px padding 保证旧字迹完全消除
                PAD = 3
                ex1 = max(0, x1 - PAD)
                ey1 = max(0, y1 - PAD)
                ex2 = min(img_cv.shape[1], x2 + PAD)
                ey2 = min(img_cv.shape[0], y2 + PAD)
                draw.rectangle([ex1, ey1, ex2, ey2], fill=bg_rgb)

                # 计算布局
                lines, font, line_gap, total_h = self._layout_text(
                    draw, trans_text, box_w, box_h, avg_h
                )

                # 垂直起点：在 box 内顶部对齐（留 2px 上边距）
                if total_h <= box_h:
                    start_y = y1 + (box_h - total_h) // 2
                else:
                    start_y = y1 + 2

                # 逐行绘制，左对齐，左侧留 2px padding
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

            # 导出
            out = io.BytesIO()
            pil_img.save(out, format="PNG")
            return out.getvalue()

        except Exception as e:
            print("ERROR in process_and_draw:", e)
            import traceback
            traceback.print_exc()
            raise e
