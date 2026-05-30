import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
import os
import io
import threading
import time
import re
import logging
import inspect

logger = logging.getLogger(__name__)


class OcrCache:
    def __init__(self, maxsize=500):
        self.maxsize = maxsize
        self.cache = {}
        self.lock = threading.Lock()
        self.order = []

    def get(self, img_md5: str):
        with self.lock:
            if img_md5 in self.cache:
                if img_md5 in self.order:
                    self.order.remove(img_md5)
                self.order.append(img_md5)
                return self.cache[img_md5]
            return None

    def set(self, img_md5: str, raw_lines: list):
        with self.lock:
            if len(self.cache) >= self.maxsize and img_md5 not in self.cache:
                if self.order:
                    oldest = self.order.pop(0)
                    self.cache.pop(oldest, None)
            self.cache[img_md5] = raw_lines
            if img_md5 in self.order:
                self.order.remove(img_md5)
            self.order.append(img_md5)


class TextTranslationCache:
    def __init__(self, maxsize=2000, ttl_seconds=86400):
        self.maxsize = maxsize
        self.ttl_seconds = ttl_seconds
        self.cache = {}
        self.order = []
        self.lock = threading.Lock()

    def get(self, key: str):
        with self.lock:
            item = self.cache.get(key)
            if not item:
                return None
            value, expire = item
            if time.time() > expire:
                self.cache.pop(key, None)
                if key in self.order:
                    self.order.remove(key)
                return None
            if key in self.order:
                self.order.remove(key)
            self.order.append(key)
            return value

    def set(self, key: str, value: dict):
        with self.lock:
            if len(self.cache) >= self.maxsize and key not in self.cache:
                if self.order:
                    oldest = self.order.pop(0)
                    self.cache.pop(oldest, None)
            self.cache[key] = (value, time.time() + self.ttl_seconds)
            if key in self.order:
                self.order.remove(key)
            self.order.append(key)


class ImageProcessor:
    def __init__(self, load_ocr: bool = True):
        self._ocr_lock = threading.Lock()
        self._ocr_infer_lock = threading.Lock()
        self._font_cache = {}
        self._font_path = None
        self._ocr_cache = OcrCache()
        self._text_translation_cache = TextTranslationCache()
        self.ocr_ready = False
        if load_ocr:
            self._ensure_ocr()
        else:
            self.ocr = None

    def _ensure_ocr(self, config: dict = None):
        cfg = config or {}
        det_db_box_thresh = cfg.get("ocr_det_db_box_thresh", 0.3)
        det_db_thresh = cfg.get("ocr_det_db_thresh", 0.2)
        det_db_unclip_ratio = cfg.get("ocr_det_db_unclip_ratio", 1.6)
        
        # 缓存当前的参数，用于判断是否需要重新初始化 OCR 模型
        current_params = (det_db_box_thresh, det_db_thresh, det_db_unclip_ratio)
        if hasattr(self, '_ocr_params') and self._ocr_params != current_params:
            self.ocr = None # 强制重新加载

        with self._ocr_lock:
            if self.ocr is None:
                start_init = time.perf_counter()
                os.environ.setdefault("FLAGS_use_mkldnn", "0")
                os.environ.setdefault("FLAGS_enable_pir_api", "0")
                from paddleocr import PaddleOCR
                sig = inspect.signature(PaddleOCR)
                params = sig.parameters
                kwargs = {"lang": "ch"}
                if "det_db_box_thresh" in params:
                    kwargs.update({
                        "enable_mkldnn": False,
                        "det_db_box_thresh": det_db_box_thresh,
                        "det_db_thresh": det_db_thresh,
                        "det_db_unclip_ratio": det_db_unclip_ratio,
                        "show_log": False,
                    })
                else:
                    kwargs.update({
                        "enable_mkldnn": False,
                        "text_det_box_thresh": det_db_box_thresh,
                        "text_det_thresh": det_db_thresh,
                        "text_det_unclip_ratio": det_db_unclip_ratio,
                        "use_doc_orientation_classify": False,
                        "use_doc_unwarping": False,
                        "use_textline_orientation": False,
                    })
                try:
                    self.ocr = PaddleOCR(**kwargs)
                except ValueError as exc:
                    if "show_log" in kwargs and "show_log" in str(exc):
                        kwargs.pop("show_log", None)
                        self.ocr = PaddleOCR(**kwargs)
                    else:
                        raise
                self._ocr_params = current_params
                try:
                    dummy_img = np.zeros((32, 32, 3), dtype=np.uint8)
                    with self._ocr_infer_lock:
                        self.ocr.ocr(dummy_img, cls=False)
                except Exception:
                    pass
                self._last_init_ms = (time.perf_counter() - start_init) * 1000
                self.ocr_ready = True
            else:
                self._last_init_ms = 0.0
                self.ocr_ready = True

    def run_ocr(self, img_cv: np.ndarray, cls: bool = True):
        self._ensure_ocr()
        with self._ocr_infer_lock:
            try:
                result = self.ocr.ocr(img_cv, cls=cls)
            except TypeError as exc:
                if "cls" in str(exc):
                    result = self.ocr.ocr(img_cv)
                else:
                    raise
            return self._normalize_ocr_result(result)

    def _normalize_ocr_result(self, result):
        if not result:
            return result
        first = result[0] if isinstance(result, list) else None
        if not isinstance(first, dict) or "rec_texts" not in first:
            return result
        lines = []
        for page in result:
            texts = page.get("rec_texts") or []
            scores = page.get("rec_scores") or []
            polys = page.get("rec_polys") or page.get("dt_polys") or []
            for text, score, poly in zip(texts, scores, polys):
                points = poly.tolist() if hasattr(poly, "tolist") else poly
                lines.append([points, [text, float(score or 0.0)]])
        return [lines]

    def _normalize_ocr_text(self, texts: list[str]) -> str:
        joined = " ".join([str(t or "") for t in texts]).strip().lower()
        return re.sub(r"[^0-9a-zA-Z\u4e00-\u9fff]+", "", joined)

    def _make_text_cache_key(self, texts: list[str], config: dict | None, target_lang: str = "zh") -> str:
        cfg = config or {}
        channel = cfg.get("active_channel", "default")
        namespace = f"{channel}:{target_lang}"
        if channel == "new-api":
            llm_cfg = cfg.get("channels", {}).get("new-api", {})
            namespace = f"new-api:{target_lang}:{llm_cfg.get('base_url', '')}:{llm_cfg.get('model', '')}"
        return f"{namespace}:{self._normalize_ocr_text(texts)}"

    def _merge_blocks(self, blocks: list[dict]) -> dict:
        x1 = min(b["rect"][0] for b in blocks)
        y1 = min(b["rect"][1] for b in blocks)
        x2 = max(b["rect"][2] for b in blocks)
        y2 = max(b["rect"][3] for b in blocks)
        avg_h = sum(float(b.get("avg_h", y2 - y1)) for b in blocks) / max(len(blocks), 1)
        return {"rect": [int(x1), int(y1), int(x2), int(y2)], "text": " ".join(b.get("text", "") for b in blocks), "avg_h": avg_h}

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
                "C:\\Windows\\Fonts\\msyh.ttc",
                "C:\\Windows\\Fonts\\msyhbd.ttc",
                "C:\\Windows\\Fonts\\simhei.ttf",
                "C:\\Windows\\Fonts\\simsun.ttc",
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
                "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
                "/usr/share/fonts/truetype/arphic/uming.ttc",
            ]
            active = next((p for p in font_paths if os.path.exists(p)), None)
            if active:
                self._font_path = active

        font = ImageFont.truetype(active, size) if active else ImageFont.load_default()
        self._font_cache[size] = font
        return font

    def _group_into_lines(self, raw_lines: list) -> list:
        if not raw_lines:
            return []
        items = []
        for line in raw_lines:
            box = line[0]
            text = line[1][0].strip()
            confidence = line[1][1]
            if confidence < 0.60:
                continue
            xs = [pt[0] for pt in box]
            ys = [pt[1] for pt in box]
            x_min, y_min, x_max, y_max = min(xs), min(ys), max(xs), max(ys)
            w = x_max - x_min
            h = y_max - y_min
            cy = y_min + h / 2.0
            items.append({"rect": [x_min, y_min, x_max, y_max], "text": text, "w": w, "h": h, "cy": cy, "confidence": confidence})
        items.sort(key=lambda b: b["rect"][1])
        virtual_blocks = []
        while items:
            current = items.pop(0)
            merged_group = [current]
            i = 0
            while i < len(items):
                candidate = items[i]
                last = merged_group[-1]
                avg_h = (last["h"] + candidate["h"]) / 2.0
                same_line = abs(last["cy"] - candidate["cy"]) <= 0.6 * avg_h
                gap_x = candidate["rect"][0] - last["rect"][2]
                horizontal_near = 0 <= gap_x <= 2.2 * avg_h
                height_similar = (max(last["h"], candidate["h"]) / max(min(last["h"], candidate["h"]), 0.001)) <= 1.5
                merged_len = sum(len(b["text"]) for b in merged_group) + len(candidate["text"])
                count_ok = len(merged_group) < 6
                if same_line and horizontal_near and height_similar and merged_len <= 80 and count_ok:
                    merged_group.append(candidate)
                    items.pop(i)
                else:
                    i += 1
            all_xs, all_ys, texts_to_join = [], [], []
            for b in merged_group:
                r = b["rect"]
                all_xs.extend([r[0], r[2]])
                all_ys.extend([r[1], r[3]])
                texts_to_join.append(b["text"])
            virtual_blocks.append({
                "rect": [int(min(all_xs)), int(min(all_ys)), int(max(all_xs)), int(max(all_ys))],
                "text": " ".join(texts_to_join),
                "avg_h": sum(b["h"] for b in merged_group) / len(merged_group),
            })
        return virtual_blocks

    def _sample_bg(self, img_cv: np.ndarray, x1: int, y1: int, x2: int, y2: int) -> tuple:
        h, w = img_cv.shape[:2]
        pad = 4
        ry1, ry2 = max(0, y1 - pad), min(h, y2 + pad)
        rx1, rx2 = max(0, x1 - pad), min(w, x2 + pad)
        if ry1 >= ry2 or rx1 >= rx2:
            return (255, 255, 255)
        region = img_cv[ry1:ry2, rx1:rx2]
        iy1, iy2 = y1 - ry1, y2 - ry1
        ix1, ix2 = x1 - rx1, x2 - rx1
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
        lum = 0.299 * bg_bgr[2] + 0.587 * bg_bgr[1] + 0.114 * bg_bgr[0]
        return (20, 20, 20) if lum > 140 else (240, 240, 240)

    def _layout_text(self, draw: ImageDraw.ImageDraw, text: str, box_w: int, box_h: int, avg_h: float) -> tuple:
        font_size = max(11, int(avg_h * 0.85))
        MIN_SIZE = 10
        for attempt_size in range(font_size, MIN_SIZE - 1, -1):
            font = self._load_font(attempt_size)
            lines = self._wrap_text(draw, text, font, box_w - 4)
            line_gap = int(attempt_size * 1.2)
            total_h = len(lines) * line_gap
            if total_h <= box_h or attempt_size <= MIN_SIZE:
                return lines, font, line_gap, total_h
        font = self._load_font(MIN_SIZE)
        lines = self._wrap_text(draw, text, font, box_w - 4)
        line_gap = int(MIN_SIZE * 1.2)
        return lines, font, line_gap, len(lines) * line_gap

    def _wrap_text(self, draw: ImageDraw.ImageDraw, text: str, font: ImageFont.FreeTypeFont, max_w: int) -> list:
        lines, current = [], ""
        for ch in text:
            test = current + ch
            try:
                w = draw.textlength(test, font=font)
            except Exception:
                w = len(test) * getattr(font, "size", 12)
            if w <= max_w:
                current = test
            else:
                if current:
                    lines.append(current)
                current = ch
        if current:
            lines.append(current)
        return lines if lines else [text]

    def process_and_draw(self, img_bytes: bytes, translator_batch_fn, config: dict = None, target_lang: str = "zh") -> tuple[bytes, dict]:
        stats = {"init_ms": 0.0, "ocr_ms": 0.0, "translate_ms": 0.0, "render_ms": 0.0, "encode_ms": 0.0, "other_ms": 0.0, "total_ms": 0.0, "ocr_blocks": 0, "translate_units": 0, "cache_hits": 0, "ocr_ready": self.ocr_ready, "ocr_cache_hit": False, "text_cache_hit": False}
        start_time = time.perf_counter()
        try:
            decode_start = time.perf_counter()
            nparr = np.frombuffer(img_bytes, np.uint8)
            img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
            decode_ms = (time.perf_counter() - decode_start) * 1000
            if img_cv is None:
                raise ValueError("图片解码失败")

            ocr_max_side = config.get("ocr_max_side", 1280) if config else 1280
            ocr_cache_enabled = config.get("ocr_cache_enabled", True) if config else True
            
            cfg = config or {}
            det_db_box_thresh = cfg.get("ocr_det_db_box_thresh", 0.3)
            det_db_thresh = cfg.get("ocr_det_db_thresh", 0.2)
            det_db_unclip_ratio = cfg.get("ocr_det_db_unclip_ratio", 1.6)
            
            import hashlib
            img_md5 = hashlib.md5(img_bytes).hexdigest()
            ocr_cache_key = f"{img_md5}_ppocr_v4_{ocr_max_side}_ch_{det_db_box_thresh}_{det_db_thresh}_{det_db_unclip_ratio}"

            self._ensure_ocr(config)
            stats["init_ms"] = getattr(self, "_last_init_ms", 0.0)
            stats["ocr_ready"] = self.ocr_ready
            ocr_start = time.perf_counter()
            cached_ocr = self._ocr_cache.get(ocr_cache_key) if ocr_cache_enabled else None
            if cached_ocr is not None:
                raw_lines = cached_ocr
                stats["ocr_cache_hit"] = True
            else:
                h, w = img_cv.shape[:2]
                max_side = max(h, w)
                ocr_img = img_cv
                scale_factor = 1.0
                if ocr_max_side > 0 and max_side > ocr_max_side:
                    scale_factor = ocr_max_side / max_side
                    ocr_img = cv2.resize(img_cv, (int(w * scale_factor), int(h * scale_factor)), interpolation=cv2.INTER_AREA)
                ocr_result = self.run_ocr(ocr_img, cls=True)
                raw_lines = ocr_result[0] if ocr_result and ocr_result[0] else []
                if raw_lines and scale_factor != 1.0:
                    for line in raw_lines:
                        for pt in line[0]:
                            pt[0] /= scale_factor
                            pt[1] /= scale_factor
                if ocr_cache_enabled:
                    self._ocr_cache.set(ocr_cache_key, raw_lines)
            stats["ocr_ms"] = (time.perf_counter() - ocr_start) * 1000

            if not raw_lines:
                stats["total_ms"] = (time.perf_counter() - start_time) * 1000
                stats["other_ms"] = max(0.0, stats["total_ms"] - stats["init_ms"] - stats["ocr_ms"] - stats["translate_ms"] - stats["render_ms"] - stats["encode_ms"])
                return img_bytes, stats
            stats["ocr_blocks"] = len(raw_lines)

            line_blocks = self._group_into_lines(raw_lines)
            stats["translate_units"] = len(line_blocks)
            original_texts = [b["text"] for b in line_blocks]

            translate_start = time.perf_counter()
            text_cache_key = self._make_text_cache_key(original_texts, config, target_lang)
            cached_translation = self._text_translation_cache.get(text_cache_key) if text_cache_key.split(":", 1)[1] else None
            if cached_translation and len(cached_translation.get("texts", [])) != len(line_blocks):
                cached_translation = None
            if cached_translation:
                stats["text_cache_hit"] = True
                stats["cache_hits"] += max(1, len(original_texts))
                cached_texts = cached_translation.get("texts", [])
                translated_texts = cached_texts
            else:
                try:
                    translated_texts = translator_batch_fn(original_texts, stats)
                except Exception as te:
                    logger.warning("Translation error: %s", te)
                    translated_texts = original_texts
                if len(translated_texts) != len(original_texts):
                    translated_texts = original_texts
                self._text_translation_cache.set(text_cache_key, {"texts": list(translated_texts), "joined": " ".join(translated_texts)})
            
            import base64
            import json
            texts_list = [{"o": o, "t": t} for o, t in zip(original_texts, translated_texts)]
            stats["texts_json"] = base64.b64encode(json.dumps(texts_list, ensure_ascii=False).encode('utf-8')).decode('ascii')
            
            stats["translate_ms"] = (time.perf_counter() - translate_start) * 1000

            render_start = time.perf_counter()
            pil_img = Image.fromarray(cv2.cvtColor(img_cv, cv2.COLOR_BGR2RGB))
            draw = ImageDraw.Draw(pil_img)
            for block, trans_text in zip(line_blocks, translated_texts):
                x1, y1, x2, y2 = block["rect"]
                box_w, box_h = x2 - x1, y2 - y1
                if box_w <= 0 or box_h <= 0:
                    continue
                bg_bgr = self._sample_bg(img_cv, x1, y1, x2, y2)
                bg_rgb = self._bg_to_rgb(bg_bgr)
                text_rgb = self._choose_text_color(bg_bgr)
                ex1, ey1 = max(0, x1 - 3), max(0, y1 - 3)
                ex2, ey2 = min(img_cv.shape[1], x2 + 3), min(img_cv.shape[0], y2 + 3)
                draw.rectangle([ex1, ey1, ex2, ey2], fill=bg_rgb)
                lines, font, line_gap, total_h = self._layout_text(draw, trans_text, box_w, box_h, block["avg_h"])
                start_y = y1 + (box_h - total_h) // 2 if total_h <= box_h else y1 + 2
                for idx, line_text in enumerate(lines):
                    line_y = start_y + idx * line_gap
                    if line_y > img_cv.shape[0]:
                        break
                    draw.text((x1 + 2, line_y), line_text, fill=text_rgb, font=font, anchor="lt", stroke_width=1 if getattr(font, "size", 0) >= 13 else 0, stroke_fill=bg_rgb)
            stats["render_ms"] = (time.perf_counter() - render_start) * 1000

            encode_start = time.perf_counter()
            out = io.BytesIO()
            pil_img.save(out, format="PNG")
            stats["encode_ms"] = decode_ms + (time.perf_counter() - encode_start) * 1000
            stats["total_ms"] = (time.perf_counter() - start_time) * 1000
            stats["other_ms"] = max(0.0, stats["total_ms"] - stats["init_ms"] - stats["ocr_ms"] - stats["translate_ms"] - stats["render_ms"] - stats["encode_ms"])
            return out.getvalue(), stats
        except Exception:
            stats["total_ms"] = (time.perf_counter() - start_time) * 1000
            stats["other_ms"] = max(0.0, stats["total_ms"] - stats["init_ms"] - stats["ocr_ms"] - stats["translate_ms"] - stats["render_ms"] - stats["encode_ms"])
            raise
