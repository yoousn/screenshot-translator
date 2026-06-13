import argparse
import contextlib
import json
import os
import re
import sys
import tempfile
import time
from pathlib import Path

from ppocr_v6 import PPOCRv6Adapter


_ENGINE_CACHE: dict[tuple[str, str, str], object] = {}
_V6_ENGINE_CACHE: dict[str, PPOCRv6Adapter] = {}


def _configure_stdout() -> None:
    try:
        sys.stdin.reconfigure(encoding="utf-8", errors="replace")
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass


def _import_rapidocr():
    try:
        from rapidocr import EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR
    except Exception as exc:
        raise RuntimeError(
            "RapidOCR is not installed. Install rapidocr and onnxruntime, or bundle rapidocr-runner.exe."
        ) from exc
    return EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR


def _version_enum(OCRVersion, version: str):
    return OCRVersion.PPOCRV4 if version.lower() == "v4" else OCRVersion.PPOCRV5


def _rec_lang_enum(LangRec, lang: str):
    mapping = {
        "ch": LangRec.CH,
        "latin": LangRec.LATIN,
        "korean": LangRec.KOREAN,
        "arabic": LangRec.ARABIC,
        "cyrillic": LangRec.CYRILLIC,
        "thai": LangRec.TH,
        "th": LangRec.TH,
    }
    return mapping.get(lang, LangRec.CH)


def _engine_cache_key(lang: str, version: str, model_root: Path | None = None) -> tuple[str, str, str]:
    root_key = ""
    if model_root:
        try:
            root_key = str(model_root.resolve())
        except Exception:
            root_key = str(model_root)
    return (lang, version, root_key)


def _det_limit_side_len() -> int:
    raw_value = os.environ.get("YSN_RAPIDOCR_DET_LIMIT_SIDE_LEN", "640")
    try:
        value = int(raw_value)
    except Exception:
        value = 640
    return max(480, min(736, value))


def _build_engine_uncached(lang: str, version: str, model_root: Path | None = None):
    EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR = _import_rapidocr()
    ocr_version = _version_enum(OCRVersion, version)
    rec_lang = _rec_lang_enum(LangRec, lang)
    params = {
        "Global.model_root_dir": str(model_root) if model_root else None,
        "Global.use_cls": False,
        "Global.log_level": "warning",
        "Det.engine_type": EngineType.ONNXRUNTIME,
        "Det.lang_type": LangDet.CH,
        "Det.model_type": ModelType.MOBILE,
        "Det.ocr_version": ocr_version,
        "Det.limit_side_len": _det_limit_side_len(),
        "Det.limit_type": "min",
        "Rec.engine_type": EngineType.ONNXRUNTIME,
        "Rec.lang_type": rec_lang,
        "Rec.model_type": ModelType.MOBILE,
        "Rec.ocr_version": ocr_version,
        "Cls.engine_type": EngineType.ONNXRUNTIME,
        "Cls.lang_type": LangRec.CH,
        "Cls.model_type": ModelType.MOBILE,
        "Cls.ocr_version": ocr_version,
    }
    if not model_root:
        params.pop("Global.model_root_dir", None)
    return RapidOCR(params=params)


def _get_engine(lang: str, version: str, model_root: Path | None = None):
    key = _engine_cache_key(lang, version, model_root)
    if key in _ENGINE_CACHE:
        return _ENGINE_CACHE[key], 0, True
    started = time.perf_counter()
    engine = _build_engine_uncached(lang, version, model_root)
    init_ms = int(round((time.perf_counter() - started) * 1000))
    _ENGINE_CACHE[key] = engine
    return engine, init_ms, False


def _build_engine(lang: str, version: str, model_root: Path | None = None):
    engine, _, _ = _get_engine(lang, version, model_root)
    return engine


def _v6_cache_key(model_root: Path | None) -> str:
    if model_root is None:
        raise RuntimeError("PP-OCRv6 requires an explicit model root")
    try:
        return str(model_root.resolve())
    except Exception:
        return str(model_root)


def _get_v6_adapter(model_root: Path | None) -> tuple[PPOCRv6Adapter, int, bool]:
    cache_key = _v6_cache_key(model_root)
    if cache_key in _V6_ENGINE_CACHE:
        return _V6_ENGINE_CACHE[cache_key], 0, True
    started = time.perf_counter()
    adapter = PPOCRv6Adapter(Path(cache_key))
    init_ms = int(round((time.perf_counter() - started) * 1000))
    _V6_ENGINE_CACHE[cache_key] = adapter
    return adapter, init_ms, False


def _box_to_ints(box) -> list[list[int]]:
    points = []
    for point in box:
        x = int(round(float(point[0])))
        y = int(round(float(point[1])))
        points.append([x, y])
    return points


def _blocks_from_result(result) -> list[dict]:
    blocks = []
    boxes = result.boxes if result.boxes is not None else []
    txts = result.txts if result.txts is not None else []
    scores = result.scores if result.scores is not None else []
    for index, text in enumerate(txts):
        normalized_text = _clean_block_text(str(text or ""))
        if not normalized_text:
            continue
        score = float(scores[index]) if index < len(scores) else 0.0
        box = _box_to_ints(boxes[index]) if index < len(boxes) else [[0, 0], [0, 0], [0, 0], [0, 0]]
        blocks.append(
            {
                "text": normalized_text,
                "confidence": score,
                "box_coords": box,
            }
        )
    return _postprocess_blocks(blocks)


def _clean_block_text(text: str) -> str:
    cleaned = text.strip()
    cleaned = re.sub(r"([A-Za-z]:)\s+([\\/])", r"\1\2", cleaned)
    cleaned = re.sub(r"([A-Z_]+=\s*[A-Za-z]:)\s+([\\/])", r"\1\2", cleaned)
    return cleaned


def _bounds(block: dict) -> tuple[float, float, float, float]:
    points = block.get("box_coords") or []
    if not points:
        return 0.0, 0.0, 0.0, 0.0
    xs = [float(point[0]) for point in points]
    ys = [float(point[1]) for point in points]
    return min(xs), min(ys), max(xs), max(ys)


def _same_visual_line(left: dict, right: dict) -> bool:
    _, left_y1, _, left_y2 = _bounds(left)
    _, right_y1, _, right_y2 = _bounds(right)
    left_h = max(1.0, left_y2 - left_y1)
    right_h = max(1.0, right_y2 - right_y1)
    vertical_overlap = min(left_y2, right_y2) - max(left_y1, right_y1)
    return vertical_overlap >= min(left_h, right_h) * 0.42


def _strip_duplicate_leading_tail(previous_text: str, current_text: str) -> str:
    current = current_text.strip()
    if not current:
        return current
    previous_letters = re.sub(r"[^A-Za-z]+", "", previous_text)
    first_match = re.match(r"^([A-Za-z]{1,3})(\s+.+)$", current)
    if not previous_letters or not first_match:
        return current
    leading_fragment = first_match.group(1)
    if leading_fragment.lower() == previous_letters[-len(leading_fragment) :].lower():
        return first_match.group(2).strip()
    return current


def _fix_japanese_long_vowel_fragment(previous_text: str, current_text: str) -> str:
    current = current_text.strip()
    if not current.startswith("一"):
        return current
    previous = previous_text.strip()
    if previous and 0x30A0 <= ord(previous[-1]) <= 0x30FF:
        return "ー" + current[1:]
    return current


def _postprocess_blocks(blocks: list[dict]) -> list[dict]:
    if len(blocks) < 2:
        return [block for block in blocks if not _is_tiny_noise_block(block)]

    fixed_by_index = {index: dict(block) for index, block in enumerate(blocks)}
    visual_lines: list[list[tuple[int, dict]]] = []

    for index, block in sorted(enumerate(blocks), key=lambda item: _bounds(item[1])[1]):
        for line in visual_lines:
            if any(_same_visual_line(line_block, block) for _, line_block in line):
                line.append((index, block))
                break
        else:
            visual_lines.append([(index, block)])

    for line in visual_lines:
        previous_index: int | None = None
        previous_block: dict | None = None
        for index, block in sorted(line, key=lambda item: _bounds(item[1])[0]):
            if previous_block is not None and previous_index is not None:
                prev_min_x, _, prev_max_x, _ = _bounds(previous_block)
                cur_min_x, _, _, _ = _bounds(block)
                line_height = max(1.0, _bounds(block)[3] - _bounds(block)[1])
                overlaps_previous = cur_min_x < prev_max_x + line_height * 0.35 and cur_min_x > prev_min_x
                current_text = str(block.get("text") or "").strip()
                if overlaps_previous:
                    stripped_text = _strip_duplicate_leading_tail(
                        str(fixed_by_index[previous_index].get("text") or ""),
                        current_text,
                    )
                    current_text = stripped_text
                current_text = _fix_japanese_long_vowel_fragment(
                    str(fixed_by_index[previous_index].get("text") or ""),
                    current_text,
                )
                if current_text != str(block.get("text") or "").strip():
                    fixed_by_index[index]["text"] = current_text
            previous_index = index
            previous_block = fixed_by_index[index]

    return [
        fixed_by_index[index]
        for index in range(len(blocks))
        if str(fixed_by_index[index].get("text") or "").strip() and not _is_tiny_noise_block(fixed_by_index[index])
    ]


def _is_tiny_noise_block(block: dict) -> bool:
    text = str(block.get("text") or "").strip()
    confidence = float(block.get("confidence") or 0.0)
    min_x, min_y, max_x, max_y = _bounds(block)
    width = max_x - min_x
    height = max_y - min_y
    return bool(re.fullmatch(r"[A-Za-z0-9]{1,2}", text)) and confidence < 0.78 and max(width, height) < 22


def _script_counts(text: str) -> dict[str, int]:
    counts = {
        "latin": 0,
        "cjk": 0,
        "hangul": 0,
        "kana": 0,
        "arabic": 0,
        "cyrillic": 0,
        "thai": 0,
    }
    for ch in text:
        code = ord(ch)
        if "A" <= ch <= "Z" or "a" <= ch <= "z":
            counts["latin"] += 1
        elif 0x4E00 <= code <= 0x9FFF:
            counts["cjk"] += 1
        elif 0x3040 <= code <= 0x30FF:
            counts["kana"] += 1
        elif 0xAC00 <= code <= 0xD7AF:
            counts["hangul"] += 1
        elif 0x0600 <= code <= 0x06FF:
            counts["arabic"] += 1
        elif 0x0400 <= code <= 0x04FF:
            counts["cyrillic"] += 1
        elif 0x0E00 <= code <= 0x0E7F:
            counts["thai"] += 1
    return counts


def _joined_text(blocks: list[dict]) -> str:
    return "\n".join(str(block.get("text") or "") for block in blocks)


def _candidate_quality(blocks: list[dict], lang: str = "ch") -> float:
    if not blocks:
        return -1000.0
    text = _joined_text(blocks)
    chars = [ch for ch in text if not ch.isspace()]
    if not chars:
        return -1000.0
    counts = _script_counts(text)
    avg_conf = sum(float(block.get("confidence") or 0.0) for block in blocks) / max(1, len(blocks))
    question_ratio = sum(1 for ch in chars if ch == "?") / max(1, len(chars))
    replacement_ratio = sum(1 for ch in chars if ch == "\ufffd") / max(1, len(chars))
    orphan_fragment_count = sum(
        1
        for word in re.findall(r"\b[A-Za-z]\b", text)
        if word.lower() not in {"a", "i"}
    )
    quality = (
        avg_conf * 100.0
        + min(len(chars), 80) * 0.05
        + min(len(blocks), 12) * 0.65
        - question_ratio * 90.0
        - replacement_ratio * 120.0
        - orphan_fragment_count * 24.0
    )
    if len(blocks) == 1 and len(chars) <= 3:
        quality -= 70.0
    elif len(blocks) == 1 and len(chars) <= 6:
        quality -= 24.0

    expected_scripts = {
        "arabic": "arabic",
        "korean": "hangul",
        "cyrillic": "cyrillic",
        "th": "thai",
        "thai": "thai",
    }
    expected_script = expected_scripts.get(lang)
    if expected_script:
        expected_count = counts.get(expected_script, 0)
        other_count = sum(value for key, value in counts.items() if key != expected_script)
        quality += min(expected_count, 48) * 2.0
        quality -= min(other_count, 48) * 1.2
        if expected_count >= 4:
            quality += 34.0
        elif expected_count == 0:
            quality -= 90.0
    elif lang == "latin":
        non_latin = counts["cjk"] + counts["kana"] + counts["hangul"] + counts["arabic"] + counts["cyrillic"] + counts["thai"]
        if counts["latin"] >= 8 and non_latin == 0:
            quality += 12.0
        elif non_latin > counts["latin"]:
            quality -= 18.0
    elif lang == "ch":
        if counts["cjk"] + counts["kana"] >= 4:
            quality += 10.0
    return quality


def _should_try_fallback(blocks: list[dict]) -> bool:
    if not blocks:
        return True
    text = "\n".join(block["text"] for block in blocks)
    chars = [ch for ch in text if not ch.isspace()]
    if not chars:
        return True
    avg_conf = sum(float(block.get("confidence") or 0.0) for block in blocks) / max(1, len(blocks))
    question_ratio = sum(1 for ch in chars if ch == "?") / max(1, len(chars))
    replacement_ratio = sum(1 for ch in chars if ch == "\ufffd") / max(1, len(chars))
    return avg_conf < 0.72 or question_ratio > 0.18 or replacement_ratio > 0.05


def _scale_block_coords(blocks: list[dict], scale: float, pad_x: int, pad_y: int) -> list[dict]:
    if scale <= 0:
        return blocks
    scaled_blocks = []
    for block in blocks:
        next_block = dict(block)
        next_coords = []
        for point in block.get("box_coords") or []:
            if len(point) < 2:
                next_coords.append(point)
                continue
            x = int(round(float(point[0]) / scale - pad_x))
            y = int(round(float(point[1]) / scale - pad_y))
            next_coords.append([max(0, x), max(0, y)])
        next_block["box_coords"] = next_coords
        scaled_blocks.append(next_block)
    return scaled_blocks


def _enhanced_image_variants(image_path: Path) -> list[dict]:
    try:
        from PIL import Image, ImageEnhance, ImageFilter, ImageOps
    except Exception:
        return []

    try:
        source = Image.open(image_path).convert("RGB")
    except Exception:
        return []

    width, height = source.size
    if width <= 0 or height <= 0:
        return []

    scale = 3.0 if max(width, height) < 700 else 2.0
    pad = 10 if max(width, height) < 700 else 6
    variants = []
    try:
        padded = ImageOps.expand(source, border=pad, fill="white")
        target_size = (
            max(1, int(round(padded.width * scale))),
            max(1, int(round(padded.height * scale))),
        )
        resampling_container = getattr(Image, "Resampling", Image)
        resampling = getattr(resampling_container, "LANCZOS", Image.BICUBIC)
        enlarged = padded.resize(target_size, resampling)
        enhanced = ImageOps.autocontrast(enlarged.convert("L"))
        enhanced = ImageEnhance.Contrast(enhanced).enhance(1.35)
        enhanced = enhanced.filter(ImageFilter.SHARPEN)
        enhanced = enhanced.convert("RGB")
        temp_dir = Path(tempfile.gettempdir()) / "ysn-screenshot-translator" / "rapidocr"
        temp_dir.mkdir(parents=True, exist_ok=True)
        variant_path = temp_dir / f"ocr-smalltext-{os.getpid()}-{time.time_ns()}.png"
        enhanced.save(variant_path)
        variants.append(
            {
                "label": "small-text-boost",
                "path": variant_path,
                "scale": scale,
                "pad_x": pad,
                "pad_y": pad,
            }
        )
    except Exception:
        return variants
    return variants


def _should_try_small_text_retry(blocks: list[dict], quality: float) -> bool:
    if not blocks:
        return True
    if quality < 62.0 and len(blocks) <= 3:
        return True
    tiny_blocks = 0
    for block in blocks:
        min_x, min_y, max_x, max_y = _bounds(block)
        if max(max_x - min_x, max_y - min_y) < 30:
            tiny_blocks += 1
    return tiny_blocks >= max(1, len(blocks) - 1) and quality < 72.0


def _run_candidate(
    image_path: Path,
    lang: str,
    version: str,
    model_root: Path | None = None,
    *,
    variant: str = "original",
    scale: float = 1.0,
    pad_x: int = 0,
    pad_y: int = 0,
    det_cache: dict | None = None,
) -> dict:
    import copy

    engine, init_ms, cache_hit = _get_engine(lang, version, model_root)
    infer_started = time.perf_counter()

    reused_det = False
    if det_cache is not None and variant in det_cache:
        det_res, cls_res, cropped_img_list, op_record, ori_img = det_cache[variant]
        reused_det = True
        
        if det_res.boxes is None or len(det_res.boxes) == 0:
            return {
                "lang": lang,
                "variant": variant,
                "blocks": [],
                "quality": -1000.0,
                "timings": {
                    "init_ms": init_ms,
                    "engine_ms": 0,
                    "rapidocr_ms": 0,
                    "cache_hit": cache_hit,
                    "reused_det": True,
                },
            }

        det_res_copy = copy.deepcopy(det_res)
        cropped_img_list_copy = copy.deepcopy(cropped_img_list)

        rec_res = engine.recognize_txt(cropped_img_list_copy)
        result = engine.build_final_output(
            ori_img, det_res_copy, cls_res, rec_res, cropped_img_list_copy, op_record
        )
        infer_ms = int(round((time.perf_counter() - infer_started) * 1000))
    else:
        ori_img = engine.load_img(str(image_path))
        img, op_record = engine.preprocess_img(ori_img)
        det_res, cls_res, rec_res, cropped_img_list = engine.run_ocr_steps(img, op_record)

        if det_cache is not None:
            det_cache[variant] = (
                copy.deepcopy(det_res),
                copy.deepcopy(cls_res),
                copy.deepcopy(cropped_img_list),
                copy.deepcopy(op_record),
                ori_img,
            )

        result = engine.build_final_output(
            ori_img, det_res, cls_res, rec_res, cropped_img_list, op_record
        )
        infer_ms = int(round((time.perf_counter() - infer_started) * 1000))

    blocks = _scale_block_coords(_blocks_from_result(result), scale, pad_x, pad_y)
    return {
        "lang": lang,
        "variant": variant,
        "blocks": blocks,
        "quality": _candidate_quality(blocks, lang),
        "timings": {
            "init_ms": init_ms,
            "engine_ms": infer_ms,
            "rapidocr_ms": int(round(float(getattr(result, "elapse", 0.0)) * 1000)),
            "cache_hit": cache_hit,
            "reused_det": reused_det,
        },
    }


def _select_candidates(mode: str) -> list[str]:
    if mode == "full":
        return ["ch", "latin", "korean", "arabic", "cyrillic", "th"]
    if mode == "latin":
        return ["latin", "ch"]
    return ["ch"]


def _append_candidate(
    candidates: list[dict],
    image_path: Path,
    lang: str,
    version: str,
    model_root: Path | None,
    *,
    variant: str = "original",
    scale: float = 1.0,
    pad_x: int = 0,
    pad_y: int = 0,
    det_cache: dict | None = None,
) -> None:
    try:
        candidates.append(
            _run_candidate(
                image_path,
                lang,
                version,
                model_root,
                variant=variant,
                scale=scale,
                pad_x=pad_x,
                pad_y=pad_y,
                det_cache=det_cache,
            )
        )
    except Exception as exc:
        candidates.append({"lang": lang, "variant": variant, "error": str(exc), "blocks": [], "quality": -1000.0})


def _best_candidate(candidates: list[dict]) -> dict:
    if not candidates:
        return {"blocks": [], "quality": -1000.0, "lang": "ch", "variant": "original"}
    return max(candidates, key=lambda item: float(item.get("quality", -1000.0)))


def run_ocr(
    image_path: Path,
    version: str,
    mode: str,
    model_root: Path | None = None,
    *,
    small_text_retry: bool = True,
) -> dict:
    if version.lower() == "v6":
        adapter, _, _ = _get_v6_adapter(model_root)
        return adapter.run(image_path)

    total_started = time.perf_counter()
    candidates: list[dict] = []
    det_cache = {}

    for lang in _select_candidates(mode):
        _append_candidate(candidates, image_path, lang, version, model_root, det_cache=det_cache)
        # 候选早停：如果当前识别的质量极高，且不需要 fallback，立刻早停
        best = _best_candidate(candidates)
        if best and not _should_try_fallback(best.get("blocks", [])) and float(best.get("quality", -1000.0)) >= 65.0:
            break

    best = _best_candidate(candidates)
    # 若 mode 为 auto 且需要 fallback，则跑 latin
    if mode == "auto" and _should_try_fallback(best.get("blocks", [])):
        _append_candidate(candidates, image_path, "latin", version, model_root, det_cache=det_cache)
        best = _best_candidate(candidates)

    # 候选早停判断：如果质量已足够高，不再进行后续的高耗时 fallback 和 small_text_retry
    can_early_exit = best and not _should_try_fallback(best.get("blocks", [])) and float(best.get("quality", -1000.0)) >= 65.0

    if not can_early_exit and small_text_retry and _should_try_small_text_retry(best.get("blocks", []), float(best.get("quality", -1000.0))):
        enhanced_variants = _enhanced_image_variants(image_path)
        try:
            for variant in enhanced_variants:
                retry_langs = ["latin", "ch"] if mode in {"auto", "latin"} else ["ch", "latin"]
                for lang in retry_langs:
                    _append_candidate(
                        candidates,
                        Path(variant["path"]),
                        lang,
                        version,
                        model_root,
                        variant=variant["label"],
                        scale=float(variant["scale"]),
                        pad_x=int(variant["pad_x"]),
                        pad_y=int(variant["pad_y"]),
                        det_cache=det_cache,
                    )
        finally:
            for variant in enhanced_variants:
                try:
                    Path(variant["path"]).unlink(missing_ok=True)
                except Exception:
                    pass

    best = _best_candidate(candidates)
    can_early_exit = best and not _should_try_fallback(best.get("blocks", [])) and float(best.get("quality", -1000.0)) >= 65.0

    fallback_langs: list[str] = []
    if not can_early_exit:
        if mode == "full":
            fallback_langs = ["latin", "korean", "arabic", "cyrillic", "th"]
        elif mode == "auto" and _should_try_fallback(best.get("blocks", [])):
            fallback_langs = ["korean", "arabic", "cyrillic", "th"]

    if fallback_langs:
        already_run = {(item.get("lang"), item.get("variant", "original")) for item in candidates}
        for lang in fallback_langs:
            if (lang, "original") not in already_run:
                _append_candidate(candidates, image_path, lang, version, model_root, det_cache=det_cache)
                best = _best_candidate(candidates)
                # Fallback 早停：如果 fallback 的语言得到了非常好的结果且有该语种字符，可以早停
                if best and best.get("lang") == lang and float(best.get("quality", -1000.0)) >= 65.0:
                    break

    best = _best_candidate(candidates)
    return {
        "status": "success",
        "engine": "rapidocr",
        "modelVersion": version,
        "selectedLang": best["lang"],
        "selectedVariant": best.get("variant", "original"),
        "blocks": best.get("blocks", []),
        "timings": {
            "total_ms": int(round((time.perf_counter() - total_started) * 1000)),
            "selected_engine_ms": best.get("timings", {}).get("engine_ms"),
            "selected_init_ms": best.get("timings", {}).get("init_ms"),
            "candidate_count": len(candidates),
        },
        "candidates": [
            {
                "lang": item.get("lang"),
                "variant": item.get("variant", "original"),
                "quality": item.get("quality"),
                "blocks": len(item.get("blocks", [])),
                "error": item.get("error"),
                "timings": item.get("timings"),
            }
            for item in candidates
        ],
    }


def run_probe(version: str, model_root: Path | None = None) -> dict:
    started = time.perf_counter()
    if version.lower() == "v6":
        adapter, init_ms, cached = _get_v6_adapter(model_root)
        payload = adapter.probe()
        payload["timings"] = {
            "total_ms": int(round((time.perf_counter() - started) * 1000)),
            "init_ms": init_ms,
            "cached": cached,
        }
        return payload
    _build_engine("ch", version, model_root)
    return {
        "status": "success",
        "engine": "rapidocr",
        "modelVersion": version,
        "probe": "engine-init",
        "timings": {"total_ms": int(round((time.perf_counter() - started) * 1000))},
    }


def run_warm_models(
    model_root: Path | None = None,
    *,
    versions: list[str] | None = None,
    langs: list[str] | None = None,
) -> dict:
    started = time.perf_counter()
    warmed = []
    errors = []
    warm_plan = {
        "v5": ["ch", "latin", "korean", "arabic", "cyrillic", "th"],
        "v4": ["ch", "latin", "korean", "arabic", "cyrillic"],
    }
    selected_versions = versions or list(warm_plan.keys())
    for version in selected_versions:
        if version == "v6":
            try:
                _get_v6_adapter(model_root)
                warmed.append({"version": version, "lang": "v6"})
            except Exception as exc:
                errors.append({"version": version, "lang": "v6", "error": str(exc)})
            continue
        selected_langs = langs or warm_plan.get(version, ["ch", "latin"])
        for lang in selected_langs:
            try:
                _build_engine(lang, version, model_root)
                warmed.append({"version": version, "lang": lang})
            except Exception as exc:
                errors.append({"version": version, "lang": lang, "error": str(exc)})
    return {
        "status": "success" if not errors else "partial",
        "engine": "rapidocr",
        "warmed": warmed,
        "errors": errors,
        "timings": {"total_ms": int(round((time.perf_counter() - started) * 1000))},
    }


def _worker_status() -> dict:
    return {
        "status": "success",
        "engine": "rapidocr-worker",
        "pid": os.getpid(),
        "cachedEngines": [
            {"lang": lang, "version": version, "modelRoot": model_root}
            for lang, version, model_root in _ENGINE_CACHE.keys()
        ],
        "cachedV6Engines": [{"version": "v6", "modelRoot": model_root} for model_root in _V6_ENGINE_CACHE.keys()],
    }


def _serve_worker() -> int:
    _configure_stdout()
    for raw_line in sys.stdin:
        raw_line = raw_line.strip().lstrip("\ufeff")
        if not raw_line:
            continue
        request_id = None
        try:
            request = json.loads(raw_line)
            request_id = request.get("id")
            method = request.get("method")
            params = request.get("params") or {}
            should_shutdown = False
            with contextlib.redirect_stdout(sys.stderr):
                if method == "ping":
                    result = {"status": "success", "engine": "rapidocr-worker", "message": "pong"}
                elif method == "status":
                    result = _worker_status()
                elif method == "warm":
                    model_root = Path(params["modelRoot"]) if params.get("modelRoot") else None
                    versions = params.get("versions")
                    if not versions and params.get("modelVersion"):
                        versions = [params["modelVersion"]]
                    langs = params.get("langs")
                    result = run_warm_models(model_root, versions=versions, langs=langs)
                elif method == "ocr":
                    image_path = Path(params["imagePath"])
                    if not image_path.exists():
                        raise RuntimeError(f"image not found: {image_path}")
                    model_root = Path(params["modelRoot"]) if params.get("modelRoot") else None
                    result = run_ocr(
                        image_path,
                        params.get("modelVersion", "v6"),
                        params.get("mode", "auto"),
                        model_root,
                        small_text_retry=bool(params.get("smallTextRetry", True)),
                    )
                elif method == "shutdown":
                    result = {"status": "success"}
                    should_shutdown = True
                else:
                    raise RuntimeError(f"unknown worker method: {method}")
            print(json.dumps({"id": request_id, "ok": True, "result": result}, ensure_ascii=False), flush=True)
            if should_shutdown:
                return 0
        except Exception as exc:
            print(json.dumps({"id": request_id, "ok": False, "error": str(exc)}, ensure_ascii=False), flush=True)
    return 0


def main() -> int:
    _configure_stdout()
    parser = argparse.ArgumentParser(description="RapidOCR JSON runner for YSN Screenshot Translator")
    parser.add_argument("--image")
    parser.add_argument("--model-version", choices=["v4", "v5", "v6"], default="v6")
    parser.add_argument("--mode", choices=["auto", "full", "latin"], default="auto")
    parser.add_argument("--model-root")
    parser.add_argument("--probe", action="store_true")
    parser.add_argument("--warm-models", action="store_true")
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--no-small-text-retry", action="store_true")
    args = parser.parse_args()
    model_root = Path(args.model_root) if args.model_root else None

    if args.worker:
        return _serve_worker()

    if args.warm_models:
        try:
            payload = run_warm_models(model_root)
            print(json.dumps(payload, ensure_ascii=False))
            return 0 if payload["status"] == "success" else 1
        except Exception as exc:
            print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False))
            return 1

    if args.probe:
        try:
            print(json.dumps(run_probe(args.model_version, model_root), ensure_ascii=False))
            return 0
        except Exception as exc:
            print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False))
            return 1

    if not args.image:
        print(json.dumps({"status": "failed", "error": "--image is required unless --probe is used"}, ensure_ascii=False))
        return 2

    image_path = Path(args.image)
    if not image_path.exists():
        print(json.dumps({"status": "failed", "error": f"image not found: {image_path}"}, ensure_ascii=False))
        return 2

    try:
        payload = run_ocr(image_path, args.model_version, args.mode, model_root, small_text_retry=not args.no_small_text_retry)
        print(json.dumps(payload, ensure_ascii=False))
        return 0
    except Exception as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
