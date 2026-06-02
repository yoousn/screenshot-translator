import argparse
import json
import re
import sys
import time
from pathlib import Path


def _configure_stdout() -> None:
    try:
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


def _build_engine(lang: str, version: str):
    EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR = _import_rapidocr()
    ocr_version = _version_enum(OCRVersion, version)
    rec_lang = _rec_lang_enum(LangRec, lang)
    params = {
        "Det.engine_type": EngineType.ONNXRUNTIME,
        "Det.lang_type": LangDet.CH,
        "Det.model_type": ModelType.MOBILE,
        "Det.ocr_version": ocr_version,
        "Rec.engine_type": EngineType.ONNXRUNTIME,
        "Rec.lang_type": rec_lang,
        "Rec.model_type": ModelType.MOBILE,
        "Rec.ocr_version": ocr_version,
        "Cls.engine_type": EngineType.ONNXRUNTIME,
        "Cls.lang_type": LangRec.CH,
        "Cls.model_type": ModelType.MOBILE,
        "Cls.ocr_version": ocr_version,
    }
    return RapidOCR(params=params)


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


def _is_latin_heavy(blocks: list[dict]) -> bool:
    counts = _script_counts(_joined_text(blocks))
    non_latin = counts["cjk"] + counts["kana"] + counts["hangul"] + counts["arabic"] + counts["cyrillic"] + counts["thai"]
    return counts["latin"] >= 8 and non_latin == 0


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


def _run_candidate(image_path: Path, lang: str, version: str) -> dict:
    started = time.perf_counter()
    engine = _build_engine(lang, version)
    init_ms = int(round((time.perf_counter() - started) * 1000))
    infer_started = time.perf_counter()
    result = engine(str(image_path))
    infer_ms = int(round((time.perf_counter() - infer_started) * 1000))
    blocks = _blocks_from_result(result)
    return {
        "lang": lang,
        "blocks": blocks,
        "quality": _candidate_quality(blocks, lang),
        "timings": {
            "init_ms": init_ms,
            "engine_ms": infer_ms,
            "rapidocr_ms": int(round(float(getattr(result, "elapse", 0.0)) * 1000)),
        },
    }


def _select_candidates(mode: str) -> list[str]:
    if mode == "full":
        return ["ch", "latin", "korean", "arabic", "cyrillic", "th"]
    if mode == "latin":
        return ["latin", "ch"]
    return ["ch"]


def run_ocr(image_path: Path, version: str, mode: str) -> dict:
    total_started = time.perf_counter()
    primary = _run_candidate(image_path, "ch", version)
    candidates = [primary]
    fallback_langs: list[str] = []
    if mode == "full":
        fallback_langs = ["latin", "korean", "arabic", "cyrillic", "th"]
    elif mode == "auto" and _should_try_fallback(primary["blocks"]):
        fallback_langs = ["latin", "korean", "arabic", "cyrillic", "th"]
    elif mode == "latin":
        fallback_langs = ["latin"]

    if fallback_langs:
        for lang in fallback_langs:
            try:
                candidates.append(_run_candidate(image_path, lang, version))
            except Exception as exc:
                candidates.append({"lang": lang, "error": str(exc), "blocks": [], "quality": -1000.0})

    best = max(candidates, key=lambda item: float(item.get("quality", -1000.0)))
    return {
        "status": "success",
        "engine": "rapidocr",
        "modelVersion": version,
        "selectedLang": best["lang"],
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
                "quality": item.get("quality"),
                "blocks": len(item.get("blocks", [])),
                "error": item.get("error"),
                "timings": item.get("timings"),
            }
            for item in candidates
        ],
    }


def run_probe(version: str) -> dict:
    started = time.perf_counter()
    _build_engine("ch", version)
    return {
        "status": "success",
        "engine": "rapidocr",
        "modelVersion": version,
        "probe": "engine-init",
        "timings": {"total_ms": int(round((time.perf_counter() - started) * 1000))},
    }


def run_warm_models() -> dict:
    started = time.perf_counter()
    warmed = []
    errors = []
    warm_plan = {
        "v5": ["ch", "latin", "korean", "arabic", "cyrillic", "th"],
        "v4": ["ch", "latin", "korean", "arabic", "cyrillic"],
    }
    for version, langs in warm_plan.items():
        for lang in langs:
            try:
                _build_engine(lang, version)
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


def main() -> int:
    _configure_stdout()
    parser = argparse.ArgumentParser(description="RapidOCR JSON runner for YSN Screenshot Translator")
    parser.add_argument("--image")
    parser.add_argument("--model-version", choices=["v4", "v5"], default="v5")
    parser.add_argument("--mode", choices=["auto", "full", "latin"], default="auto")
    parser.add_argument("--probe", action="store_true")
    parser.add_argument("--warm-models", action="store_true")
    args = parser.parse_args()

    if args.warm_models:
        try:
            payload = run_warm_models()
            print(json.dumps(payload, ensure_ascii=False))
            return 0 if payload["status"] == "success" else 1
        except Exception as exc:
            print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False))
            return 1

    if args.probe:
        try:
            print(json.dumps(run_probe(args.model_version), ensure_ascii=False))
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
        payload = run_ocr(image_path, args.model_version, args.mode)
        print(json.dumps(payload, ensure_ascii=False))
        return 0
    except Exception as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
