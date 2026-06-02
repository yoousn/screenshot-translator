import abc
import requests
import json
import urllib.parse
import hashlib
import random
import logging
import re
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from security import normalize_public_base_url, request_public_url

logger = logging.getLogger(__name__)

_HAS_CHINESE_RE = re.compile(r"[\u3400-\u9fff]")
_HAS_LATIN_RE = re.compile(r"[A-Za-z]{2,}")
_HAS_NON_CHINESE_TRANSLATABLE_RE = re.compile(r"[\u0400-\u052f\u0600-\u06ff\u0e00-\u0e7f\u3040-\u30ff\uac00-\ud7af]")
_TRANS_LABEL_RE = re.compile(r"[A-Za-z]{2,}|[\u0400-\u052f]{2,}|[\u0600-\u06ff]{2,}|[\u0e00-\u0e7f]{2,}|[\u3040-\u30ff]{2,}|[\uac00-\ud7af]{2,}")
_FILE_EXT_RE = re.compile(r"\.(?:exe|dll|json|md|markdown|txt|onnx|yaml|yml|toml|rs|ts|tsx|js|jsx|mjs|py|ps1|bat|cmd|png|jpe?g|webp|gif|zip|7z|msi|nsi|lock|log)$", re.IGNORECASE)
_PATH_LIKE_RE = re.compile(r"(?:^[A-Za-z]:[\\/]|[\\/]|^\.\.?[\\/]|~[\\/])")
_COMMAND_FLAG_RE = re.compile(r"^-{1,2}[\w-]+(?:[=:][^\s]+)?$")
_ENV_ASSIGNMENT_RE = re.compile(r"^[A-Z_][A-Z0-9_]*=.+$")
_COMMAND_LINE_MARKER_RE = re.compile(r"(?:&&|\|\||\s-{1,2}[\w-]+)")
_PACKAGE_LIKE_RE = re.compile(r"^(?:@[\w.-]+/)?[\w.-]+(?:/[\w.-]+)+$")
_UPPER_IDENTIFIER_RE = re.compile(r"^[A-Z0-9][A-Z0-9_.-]*[_./-][A-Z0-9_.-]*$")
_TECHNICAL_SAFE_TEXT_RE = re.compile(r"^[\w .:@~+\\/\-=&|]+$")
_PROTECTED_EXACT_TERMS = {
    "path",
    "windows",
    "ocr",
    "onnx",
    "rapidocr",
    "paddleocr-json",
    "ysn ocr runtime",
    "ctrl+d",
    "ctrl+q",
}

_LATIN_DIACRITIC_RE = re.compile(r"[À-ÖØ-öø-ÿ]")
_NON_ENGLISH_LATIN_RE = re.compile(
    r"\b(?:abrir|antes|guardar|vista|previa|ouvrir|aperçu|apercu|avant|enregistrer|paramètres|parametres|fichier|fenêtre|fenetre|actualizar|cancelar|copiar|configuración|configuracion|de|des|du|del|para|por|con|sin)\b",
    re.IGNORECASE,
)


def has_likely_non_english_latin_text(text: str) -> bool:
    text = " ".join((text or "").strip().split())
    if not re.search(r"[A-Za-z]{2,}", text):
        return False
    return bool(_LATIN_DIACRITIC_RE.search(text) or _NON_ENGLISH_LATIN_RE.search(text))


def _trim_token_punctuation(text: str) -> str:
    return (text or "").strip().strip("`\"'([{<").rstrip("`\"'])}>.,;:")


def is_protected_technical_token(raw: str) -> bool:
    token = _trim_token_punctuation(raw)
    if not token:
        return False
    lower = token.lower()
    if lower in _PROTECTED_EXACT_TERMS:
        return True
    if re.match(r"^ctrl\+[a-z0-9]$", token, re.IGNORECASE):
        return True
    if _ENV_ASSIGNMENT_RE.match(token):
        return True
    if _COMMAND_FLAG_RE.match(token):
        return True
    if _FILE_EXT_RE.search(token):
        return True
    if _PATH_LIKE_RE.search(token) and _TECHNICAL_SAFE_TEXT_RE.match(token):
        return True
    if _PACKAGE_LIKE_RE.match(token):
        return True
    if _UPPER_IDENTIFIER_RE.match(token) and token == token.upper():
        return True
    return False


def is_likely_protected_technical_text(text: str) -> bool:
    normalized = " ".join((text or "").strip().split())
    if not normalized or _HAS_CHINESE_RE.search(normalized):
        return False
    if is_protected_technical_token(normalized):
        return True

    tokens = [item for item in normalized.split() if item]
    if len(tokens) <= 1:
        return False

    has_path_or_file_marker = _PATH_LIKE_RE.search(normalized) or _FILE_EXT_RE.search(normalized)
    if has_path_or_file_marker and _TECHNICAL_SAFE_TEXT_RE.match(normalized):
        return True
    has_command_line_marker = _ENV_ASSIGNMENT_RE.match(tokens[0] or "") or _COMMAND_LINE_MARKER_RE.search(normalized)
    if has_command_line_marker and _TECHNICAL_SAFE_TEXT_RE.match(normalized):
        return True

    return all(is_protected_technical_token(token) for token in tokens)


def should_preserve_without_translation(text: str, target_lang: str) -> bool:
    normalized = " ".join((text or "").strip().split())
    if not normalized:
        return True
    if is_likely_protected_technical_text(normalized):
        return True
    if (
        target_lang in {"zh", "zh-CN"}
        and _HAS_CHINESE_RE.search(normalized)
        and not _HAS_LATIN_RE.search(normalized)
        and not _HAS_NON_CHINESE_TRANSLATABLE_RE.search(normalized)
    ):
        return True
    return not _TRANS_LABEL_RE.search(normalized)


def detect_source_lang_hint(text: str, fallback: str = "auto") -> str:
    if re.search(r"[\uac00-\ud7af]", text or ""):
        return "ko"
    if re.search(r"[\u0600-\u06ff]", text or ""):
        return "ar"
    if re.search(r"[\u3040-\u30ff]", text or ""):
        return "ja"
    if re.search(r"[\u0400-\u052f]", text or ""):
        return "ru"
    if re.search(r"[\u0e00-\u0e7f]", text or ""):
        return "th"
    if fallback == "en" and has_likely_non_english_latin_text(text):
        return "auto"
    return fallback or "auto"

def should_group_by_source_hints(texts: list[str], source_lang: str) -> bool:
    hints = {detect_source_lang_hint(text, "auto") for text in texts}
    if source_lang == "auto":
        return len(hints) > 1 or (hints and "auto" not in hints)
    hints = {detect_source_lang_hint(text, source_lang) for text in texts}
    return len(hints) > 1 or (hints and source_lang not in hints)

# 使用全局共享的 requests Session 保持 Keep-Alive 长连接，免去每次 TLS 握手的开销
_shared_session = requests.Session()
_shared_session.trust_env = False
# 适当调整连接池大小
adapter = requests.adapters.HTTPAdapter(pool_connections=10, pool_maxsize=20)
_shared_session.mount("http://", adapter)
_shared_session.mount("https://", adapter)

import time
import threading

class TranslationCache:
    def __init__(self, maxsize=5000, ttl_seconds=86400):
        self.maxsize = maxsize
        self.ttl_seconds = ttl_seconds
        self.lock = threading.RLock()
        self.cache = {} # key -> (value, expire_time)
        self.access_order = [] # key access ordering (LRU)

    def _normalize_text(self, text: str) -> str:
        # 去除首尾空白，折叠连续空白字符
        return " ".join(text.strip().split())

    def make_key(self, text: str, src_lang: str, dst_lang: str, channel: str, version: str) -> tuple:
        return (self._normalize_text(text), src_lang, dst_lang, channel, version)

    def get(self, key: tuple):
        with self.lock:
            if key not in self.cache:
                return None
            val, expire = self.cache[key]
            if time.time() > expire:
                # Expired
                self.cache.pop(key, None)
                if key in self.access_order:
                    self.access_order.remove(key)
                return None
            # Refresh LRU ordering
            if key in self.access_order:
                self.access_order.remove(key)
            self.access_order.append(key)
            return val

    def set(self, key: tuple, value: str):
        with self.lock:
            # Evict oldest if full
            if len(self.cache) >= self.maxsize and key not in self.cache:
                if self.access_order:
                    oldest = self.access_order.pop(0)
                    self.cache.pop(oldest, None)

            expire = time.time() + self.ttl_seconds
            self.cache[key] = (value, expire)
            if key in self.access_order:
                self.access_order.remove(key)
            self.access_order.append(key)

# 全局共享翻译缓存实例 (maxsize=5000, TTL=24h)
GLOBAL_TRANSLATE_CACHE = TranslationCache(maxsize=5000, ttl_seconds=86400)


def _load_translation_glossary() -> dict:
    candidates = [
        Path(__file__).with_name("translationGlossary.json"),
        Path(__file__).resolve().parents[1] / "tauri-client" / "src" / "utils" / "translationGlossary.json",
    ]
    for path in candidates:
        try:
            with path.open("r", encoding="utf-8") as file:
                return json.load(file)
        except FileNotFoundError:
            continue
        except Exception as exc:
            logger.warning("Failed to load translation glossary %s: %s", path, exc)
    return {"version": "fallback", "zh": {"ui": {}}}


TRANSLATION_GLOSSARY = _load_translation_glossary()
ZH_UI_GLOSSARY = TRANSLATION_GLOSSARY.get("zh", {}).get("ui", {})


def get_translation_runtime_metadata(active_channel: str = "google") -> dict:
    return {
        "glossary_version": TRANSLATION_GLOSSARY.get("version", "unknown"),
        "glossary_loaded": bool(ZH_UI_GLOSSARY),
        "glossary_terms": len(ZH_UI_GLOSSARY),
        "quality_flags": {
            "short_ui_glossary": bool(ZH_UI_GLOSSARY),
            "latin_non_english_auto_source": True,
            "multiline_block_preserved": True,
            "technical_identifier_preservation": True,
            "google_free_low_quality_risk": active_channel == "google",
        },
    }


def lookup_short_ui_glossary(text: str, source_lang: str, target_lang: str) -> str | None:
    if target_lang not in {"zh", "zh-CN"}:
        return None
    if source_lang not in {"auto", "en"}:
        return None
    normalized = " ".join((text or "").strip().lower().split())
    return ZH_UI_GLOSSARY.get(normalized)


class BaseTranslator(abc.ABC):
    def __init__(self):
        self.session = _shared_session

    @abc.abstractmethod
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        pass

    def cache_namespace(self) -> str:
        return self.__class__.__name__.lower().replace("translator", "")

    def _ensure_stats_ref(self, stats_ref: dict = None) -> dict | None:
        if stats_ref is None:
            return None
        stats_ref.setdefault("cache_hits", 0)
        stats_ref.setdefault("provider_misses", 0)
        stats_ref.setdefault("request_duplicates", 0)
        stats_ref.setdefault("preserved_hits", 0)
        return stats_ref

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []

        stats_ref = self._ensure_stats_ref(stats_ref)
        results = [None] * len(texts)
        miss_texts = []
        miss_keys = []
        miss_slots = []
        scheduled_misses = {}

        channel_name = self.cache_namespace()
        # 后续如果有版本区分可从配置读取，目前固定 "1.0"
        version = "1.0"

        for idx, text in enumerate(texts):
            if should_preserve_without_translation(text, target_lang):
                results[idx] = text
                if stats_ref is not None:
                    stats_ref["preserved_hits"] += 1
                continue
            glossary_val = lookup_short_ui_glossary(text, source_lang, target_lang)
            if glossary_val is not None:
                results[idx] = glossary_val
                if stats_ref is not None:
                    stats_ref["cache_hits"] += 1
                continue
            key = GLOBAL_TRANSLATE_CACHE.make_key(text, source_lang, target_lang, channel_name, version)
            cached_val = GLOBAL_TRANSLATE_CACHE.get(key)
            if cached_val is not None:
                results[idx] = cached_val
                if stats_ref is not None:
                    stats_ref["cache_hits"] += 1
            else:
                scheduled_index = scheduled_misses.get(key)
                if scheduled_index is not None:
                    miss_slots[scheduled_index].append(idx)
                    if stats_ref is not None:
                        stats_ref["request_duplicates"] += 1
                    continue
                scheduled_misses[key] = len(miss_texts)
                miss_keys.append(key)
                miss_texts.append(text)
                miss_slots.append([idx])

        if miss_texts:
            if stats_ref is not None:
                stats_ref["provider_misses"] += len(miss_texts)
            translated_misses = self._do_translate_batch(miss_texts, source_lang, target_lang, stats_ref)
            if len(translated_misses) != len(miss_texts):
                translated_misses = [""] * len(miss_texts)

            for key, slots, trans_val in zip(miss_keys, miss_slots, translated_misses):
                for idx in slots:
                    results[idx] = trans_val
                # 写入缓存
                if trans_val:
                    GLOBAL_TRANSLATE_CACHE.set(key, trans_val)

        return [item or "" for item in results]

    def _do_translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []
        with ThreadPoolExecutor(max_workers=8) as executor:
            futures = [executor.submit(self.translate, text, source_lang, target_lang) for text in texts]
            return [f.result() for f in futures]

class GoogleTranslator(BaseTranslator):
    def _do_translate_script_groups(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        groups = {}
        for index, text in enumerate(texts):
            hint = detect_source_lang_hint(text, source_lang)
            groups.setdefault(hint, []).append((index, text))

        results = [""] * len(texts)
        with ThreadPoolExecutor(max_workers=min(4, max(1, len(groups)))) as executor:
            futures = {
                executor.submit(self._do_translate_batch, [text for _, text in items], hint, target_lang, stats_ref): items
                for hint, items in groups.items()
            }
            for future, items in futures.items():
                translated_items = future.result()
                if len(translated_items) != len(items):
                    translated_items = [""] * len(items)
                for (original_index, _), translated in zip(items, translated_items):
                    results[original_index] = translated
        return results

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        source_lang = detect_source_lang_hint(text, source_lang)
        url = f"https://translate.googleapis.com/translate_a/single?client=gtx&sl={source_lang}&tl={target_lang}&dt=t&q={urllib.parse.quote(text)}"
        response = self.session.get(url, timeout=5)
        if response.status_code == 200:
            try:
                res_json = response.json()
                if isinstance(res_json, list) and len(res_json) > 0 and isinstance(res_json[0], list):
                    return "".join([part[0] for part in res_json[0] if part[0]])
                raise ValueError("Unexpected JSON response structure from Google Translate")
            except (IndexError, TypeError, KeyError, ValueError) as e:
                raise Exception(f"Google translate response parsing failed: {e}")
        raise Exception(f"Google translate failed: status {response.status_code}")

    def _do_translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        """
        优化：大厂同款批量翻译。将所有行合并成一次 POST 请求发送给 Google，
        利用 Google 对换行符 \n 的保留特性，在单次往返中取得全部翻译，
        最后进行行对齐。如果行数不符，平滑降级到并发线程池。
        """
        if not texts:
            return []

        # 1. 预处理文本，去掉行内换行防止干扰分行逻辑
        if should_group_by_source_hints(texts, source_lang):
            return self._do_translate_script_groups(texts, source_lang, target_lang, stats_ref)
        if any("\n" in text for text in texts):
            return super()._do_translate_batch(texts, source_lang, target_lang, stats_ref)

        cleaned_texts = [t.replace('\n', ' ').strip() for t in texts]
        query = "\n".join(cleaned_texts)

        url = "https://translate.googleapis.com/translate_a/single"
        data = {
            "client": "gtx",
            "sl": source_lang,
            "tl": target_lang,
            "dt": "t",
            "q": query
        }

        try:
            # 使用 Post 请求，避免 Get 请求由于文本过长导致超长报错
            response = self.session.post(url, data=data, timeout=8)
            if response.status_code == 200:
                res_json = response.json()
                if isinstance(res_json, list) and len(res_json) > 0 and isinstance(res_json[0], list):
                    # 组合成完整的翻译文本
                    translated_full = "".join([part[0] for part in res_json[0] if part[0]])
                    # 按行切割
                    translated_lines = translated_full.splitlines()

                    # 校验行数是否完全对应
                    if len(translated_lines) == len(texts):
                        return translated_lines
                    else:
                        logger.warning(
                            "[Google Batch] 翻译行数不匹配: 期望 %d 行，实际返回 %d 行。正在降级为线程池并发翻译...",
                            len(texts), len(translated_lines)
                        )
        except Exception as e:
            logger.warning("[Google Batch] 批量翻译请求失败: %s。正在降级为线程池并发翻译...", e)

        # 2. 降级兜底：使用基类的多线程并发请求，保证稳定性
        return super()._do_translate_batch(texts, source_lang, target_lang, stats_ref)

class LLMTranslator(BaseTranslator):
    SEGMENT_MARKER_START = "\uE000"
    SEGMENT_MARKER_END = "\uE001"

    def __init__(self, base_url: str, api_key: str, model: str):
        super().__init__()
        self.base_url = normalize_public_base_url(base_url)
        self.api_key = api_key
        self.model = model

    def cache_namespace(self) -> str:
        parsed = urllib.parse.urlparse(self.base_url)
        host = parsed.hostname or self.base_url
        return f"llm:{host}:{self.model}"

    def _target_language_name(self, target_lang: str) -> str:
        language_names = {
            "zh": "Simplified Chinese",
            "en": "English",
            "ja": "Japanese",
            "ko": "Korean",
            "fr": "French",
            "de": "German",
            "es": "Spanish",
        }
        return language_names.get(target_lang, target_lang or "Simplified Chinese")

    def _segment_marker(self, idx: int) -> str:
        return f"{self.SEGMENT_MARKER_START}{idx}{self.SEGMENT_MARKER_END}"

    def _pack_segments(self, texts: list[str]) -> str:
        return "\n".join([f"{self._segment_marker(idx)}{text}" for idx, text in enumerate(texts)])

    def _strip_markdown_fence(self, content: str) -> str:
        content = (content or "").strip()
        if content.startswith("```"):
            lines = content.splitlines()
            if lines and lines[0].startswith("```"):
                lines = lines[1:]
            if lines and lines[-1].startswith("```"):
                lines = lines[:-1]
            content = "\n".join(lines).strip()
        return content

    def _parse_segment_response(self, content: str, expected_count: int) -> dict[int, str]:
        content = self._strip_markdown_fence(content)
        start = re.escape(self.SEGMENT_MARKER_START)
        end = re.escape(self.SEGMENT_MARKER_END)
        pattern = re.compile(rf"{start}(\d+){end}\s*(.*?)(?=\s*{start}\d+{end}|$)", re.DOTALL)
        matches = pattern.findall(content)
        parsed = {int(idx_str): body.strip() for idx_str, body in matches if body.strip()}
        expected = set(range(expected_count))
        if len(matches) != expected_count or set(parsed.keys()) != expected:
            logger.warning(
                "LLM segment response failed validation: expected indexes %s, got indexes %s, match_count=%d",
                sorted(expected), sorted(parsed.keys()), len(matches)
            )
            return {}
        return parsed

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        target_language = self._target_language_name(target_lang)
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": f"You are a translation assistant. Translate the following text into {target_language}. Translate every meaningful English word or phrase, including short UI labels, list items, buttons, and mixed Chinese-English text. Preserve line meaning and do not skip short interface text. Keep product names, commands, package names, file paths, flags, PATH, Windows, OCR, ONNX, RapidOCR, PaddleOCR-json, and .exe identifiers unchanged when they are technical identifiers. Output ONLY the translated text, do not include commentary, explanations, or quotes."},
                {"role": "user", "content": text}
            ],
            "temperature": 0.3
        }
        res = request_public_url(self.session, "POST", f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=10)
        if res.status_code == 200:
            return res.json()["choices"][0]["message"]["content"].strip()
        raise Exception(f"LLM translation failed: {res.text}")

    def _do_translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []
        target_language = self._target_language_name(target_lang)

        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }

        # 1. 使用私有区标记将多行合并成一个文本块，避免与原文/译文中的 <SEG1> 等普通文本冲突
        packed_input = self._pack_segments(texts)
        prompt = (
            f"You are a translation assistant. Translate each segment marked with private-use markers like {self._segment_marker(0)} into {target_language}.\n"
            "Translate every meaningful English word or phrase, including short UI labels and mixed Chinese-English text. Keep product names, commands, package names, file paths, and flags unchanged when they are technical identifiers.\n"
            "You MUST keep each exact marker at the start of its translated segment, preserve order, and output the same number of segments as input.\n"
            "Output ONLY the translated segments with their markers. Do not include any extra descriptions, markdown blocks, formatting or explanations."
        )
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": prompt},
                {"role": "user", "content": packed_input}
            ],
            "temperature": 0.2
        }

        parsed = {}
        try:
            res = request_public_url(self.session, "POST", f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=12)
            if res.status_code == 200:
                content = res.json()["choices"][0]["message"]["content"].strip()
                parsed = self._parse_segment_response(content, len(texts))
        except Exception as e:
            logger.warning("LLM segment-based batch translation failed: %s", e)

        # 2. 精准缺失补偿与兜底 (以多线程并发重试缺失索引)
        final_results = [None] * len(texts)
        missing_indices = []

        for idx in range(len(texts)):
            if idx in parsed and parsed[idx]:
                final_results[idx] = parsed[idx]
            else:
                missing_indices.append(idx)

        if missing_indices:
            logger.warning(
                "[LLM Segment Batch] 检测到 %d 个片段翻译缺失，正在进行精准多线程并发补偿...",
                len(missing_indices)
            )
            with ThreadPoolExecutor(max_workers=8) as executor:
                futures = {
                    idx: executor.submit(self.translate, texts[idx], source_lang, target_lang)
                    for idx in missing_indices
                }
                for idx, fut in futures.items():
                    try:
                        final_results[idx] = fut.result()
                    except Exception as fe:
                        logger.error(f"[LLM Precision Fallback] 补偿翻译索引 {idx} 失败: {fe}")
                        final_results[idx] = ""

        return final_results

class BaiduTranslator(BaseTranslator):
    def __init__(self, app_id: str, secret_key: str):
        super().__init__()
        self.app_id = app_id
        self.secret_key = secret_key

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        salt = str(random.randint(32768, 65536))
        sign_str = self.app_id + text + salt + self.secret_key
        sign = hashlib.md5(sign_str.encode('utf-8')).hexdigest()

        from_lang = "auto" if source_lang == "auto" else source_lang
        to_lang = "zh" if target_lang == "zh" else target_lang

        url = f"https://fanyi-api.baidu.com/api/trans/vip/translate?q={urllib.parse.quote(text)}&from={from_lang}&to={to_lang}&appid={self.app_id}&salt={salt}&sign={sign}"
        res = self.session.get(url, timeout=5)
        if res.status_code == 200:
            res_json = res.json()
            if "error_code" in res_json:
                raise Exception(f"Baidu translation API error: {res_json['error_msg']}")
            return "".join([item["dst"] for item in res_json["trans_result"]])
        raise Exception(f"Baidu request failed: status {res.status_code}")

    def _do_translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []
        cleaned_texts = [t.replace('\n', ' ').strip() for t in texts]
        query = "\n".join(cleaned_texts)

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
        try:
            res = self.session.post(url, data=data, timeout=8)
            if res.status_code == 200:
                res_json = res.json()
                if "error_code" in res_json:
                    raise Exception(f"Baidu translation API error: {res_json['error_msg']}")
                trans_result = res_json.get("trans_result", [])
                if len(trans_result) == len(texts):
                    return [item["dst"] for item in trans_result]
        except Exception as e:
            logger.warning("Baidu batch translation failed: %s", e)
        return super()._do_translate_batch(texts, source_lang, target_lang, stats_ref)

