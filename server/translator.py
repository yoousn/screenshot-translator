import abc
import os
import requests
import json
import urllib.parse
import hashlib
import random
import logging
import re
from collections import OrderedDict
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from security import normalize_public_base_url, normalize_relay_base_url, request_public_url, request_relay_url
from translation_prompt import DEFAULT_LLM_TRANSLATION_DOMAIN, DEFAULT_LLM_TRANSLATION_PROMPT

logger = logging.getLogger(__name__)


def _env_float(name: str, default: float) -> float:
    try:
        value = float(os.environ.get(name, ""))
        return value if value > 0 else default
    except (TypeError, ValueError):
        return default


GOOGLE_SINGLE_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_GOOGLE_SINGLE_TIMEOUT", 3.5)
GOOGLE_BATCH_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_GOOGLE_BATCH_TIMEOUT", 4.5)
GOOGLE_BATCH_BUDGET_SECONDS = _env_float("SS_TRANSLATOR_GOOGLE_BATCH_BUDGET", 8.0)
BAIDU_SINGLE_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_BAIDU_SINGLE_TIMEOUT", 4.0)
BAIDU_BATCH_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_BAIDU_BATCH_TIMEOUT", 5.5)
DEEPL_BATCH_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_DEEPL_BATCH_TIMEOUT", 8.0)
LLM_SINGLE_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_LLM_SINGLE_TIMEOUT", 5.0)
LLM_BATCH_TIMEOUT_SECONDS = _env_float("SS_TRANSLATOR_LLM_BATCH_TIMEOUT", 7.5)
LLM_BATCH_BUDGET_SECONDS = _env_float("SS_TRANSLATOR_LLM_BATCH_BUDGET", 8.5)
LLM_FALLBACK_MAX_WORKERS = max(1, int(_env_float("SS_TRANSLATOR_LLM_FALLBACK_WORKERS", 4)))


def _seconds_left(deadline: float, cap: float) -> float:
    return max(0.0, min(cap, deadline - time.perf_counter()))


def _increment_stat(stats_ref: dict | None, key: str, value: int = 1) -> None:
    if stats_ref is not None:
        stats_ref[key] = stats_ref.get(key, 0) + value

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
        self.cache = OrderedDict() # key -> (value, expire_time), newest at the end

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
            if time.perf_counter() > expire:
                # Expired
                self.cache.pop(key, None)
                return None
            # Refresh LRU ordering
            self.cache.move_to_end(key)
            return val

    def set(self, key: tuple, value: str):
        with self.lock:
            # Evict oldest if full
            if len(self.cache) >= self.maxsize and key not in self.cache:
                self.cache.popitem(last=False)

            expire = time.perf_counter() + self.ttl_seconds
            self.cache[key] = (value, expire)
            self.cache.move_to_end(key)

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
        stats_ref.setdefault("provider_failures", 0)
        stats_ref.setdefault("provider_fallbacks", 0)
        stats_ref.setdefault("provider_batch_ms", 0)
        stats_ref.setdefault("provider_fallback_ms", 0)
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
        response = self.session.get(url, timeout=GOOGLE_SINGLE_TIMEOUT_SECONDS)
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

        budget_deadline = time.perf_counter() + GOOGLE_BATCH_BUDGET_SECONDS
        try:
            # 使用 Post 请求，避免 Get 请求由于文本过长导致超长报错
            batch_started = time.perf_counter()
            response = self.session.post(url, data=data, timeout=GOOGLE_BATCH_TIMEOUT_SECONDS)
            _increment_stat(stats_ref, "provider_batch_ms", int((time.perf_counter() - batch_started) * 1000))
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
            _increment_stat(stats_ref, "provider_failures")
            logger.warning("[Google Batch] 批量翻译请求失败: %s。正在降级为线程池并发翻译...", e)

        # 2. 降级兜底：使用基类的多线程并发请求，保证稳定性
        remaining = _seconds_left(budget_deadline, GOOGLE_SINGLE_TIMEOUT_SECONDS)
        if remaining < 0.75:
            _increment_stat(stats_ref, "provider_failures", len(texts))
            return [""] * len(texts)
        fallback_started = time.perf_counter()
        result = super()._do_translate_batch(texts, source_lang, target_lang, stats_ref)
        _increment_stat(stats_ref, "provider_fallbacks", len(texts))
        _increment_stat(stats_ref, "provider_fallback_ms", int((time.perf_counter() - fallback_started) * 1000))
        return result

class LLMTranslator(BaseTranslator):
    SEGMENT_SEPARATOR = "\n%%\n"

    def __init__(self, base_url: str, api_key: str, model: str, prompt_template: str = "", translation_domain: str = "", allow_private_base_url: bool = False):
        super().__init__()
        self.allow_private_base_url = allow_private_base_url
        self.base_url = normalize_relay_base_url(base_url) if allow_private_base_url else normalize_public_base_url(base_url)
        self.api_key = api_key
        self.model = model
        self.prompt_template = (prompt_template or DEFAULT_LLM_TRANSLATION_PROMPT).strip()
        self.translation_domain = (translation_domain or DEFAULT_LLM_TRANSLATION_DOMAIN).strip()

    def cache_namespace(self) -> str:
        parsed = urllib.parse.urlparse(self.base_url)
        host = parsed.hostname or self.base_url
        prompt_hash = hashlib.sha256(f"{self.prompt_template}\n{self.translation_domain}".encode("utf-8")).hexdigest()[:12]
        return f"llm:{host}:{self.model}:{prompt_hash}"

    def _target_language_name(self, target_lang: str) -> str:
        language_names = {
            "zh": "Simplified Chinese",
            "zh-CN": "Simplified Chinese",
            "zh-TW": "Traditional Chinese",
            "en": "English",
            "ja": "Japanese",
            "ko": "Korean",
            "fr": "French",
            "de": "German",
            "es": "Spanish",
            "pt": "Portuguese",
            "it": "Italian",
            "ru": "Russian",
            "ar": "Arabic",
            "th": "Thai",
            "tr": "Turkish",
        }
        return language_names.get(target_lang, target_lang or "Simplified Chinese")

    def _source_language_name(self, source_lang: str) -> str:
        if not source_lang or source_lang == "auto":
            return "auto-detected source language"
        return self._target_language_name(source_lang)

    def _render_prompt(self, source_lang: str, target_lang: str) -> str:
        prompt = self.prompt_template or DEFAULT_LLM_TRANSLATION_PROMPT
        return (
            prompt
            .replace("{{SOURCE_LANGUAGE}}", self._source_language_name(source_lang))
            .replace("{{TARGET_LANGUAGE}}", self._target_language_name(target_lang))
            .replace("{{TRANSLATION_DOMAIN}}", self.translation_domain or DEFAULT_LLM_TRANSLATION_DOMAIN)
        )

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

    def _pack_percent_segments(self, texts: list[str]) -> str:
        return self.SEGMENT_SEPARATOR.join(texts)

    def _parse_percent_segments(self, content: str, expected_count: int) -> list[str]:
        content = self._strip_markdown_fence(content)
        parts = re.split(r"(?m)^\s*%%\s*$", content)
        parts = [part.strip() for part in parts]
        if len(parts) != expected_count:
            logger.warning(
                "LLM %% segment response failed validation: expected %d segments, got %d",
                expected_count, len(parts)
            )
            return []
        return parts

    def _translate_one(self, text: str, source_lang: str, target_lang: str, timeout_seconds: float) -> str:
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": self._render_prompt(source_lang, target_lang)},
                {"role": "user", "content": text}
            ],
            "temperature": 0.3
        }
        request_fn = request_relay_url if self.allow_private_base_url else request_public_url
        res = request_fn(self.session, "POST", f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=timeout_seconds)
        if res.status_code == 200:
            return res.json()["choices"][0]["message"]["content"].strip()
        raise Exception(f"LLM translation failed: {res.text}")

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        return self._translate_one(text, source_lang, target_lang, LLM_SINGLE_TIMEOUT_SECONDS)

    def _do_translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }

        packed_input = self._pack_percent_segments(texts)
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": self._render_prompt(source_lang, target_lang)},
                {"role": "user", "content": packed_input}
            ],
            "temperature": 0.2
        }

        parsed = {}
        batch_failed = False
        budget_deadline = time.perf_counter() + LLM_BATCH_BUDGET_SECONDS
        try:
            request_fn = request_relay_url if self.allow_private_base_url else request_public_url
            batch_started = time.perf_counter()
            res = request_fn(self.session, "POST", f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=LLM_BATCH_TIMEOUT_SECONDS)
            _increment_stat(stats_ref, "provider_batch_ms", int((time.perf_counter() - batch_started) * 1000))
            if res.status_code == 200:
                content = res.json()["choices"][0]["message"]["content"].strip()
                parsed = {idx: value for idx, value in enumerate(self._parse_percent_segments(content, len(texts)))}
            else:
                batch_failed = True
                _increment_stat(stats_ref, "provider_failures")
                logger.warning("LLM segment-based batch translation returned status %s", res.status_code)
        except Exception as e:
            batch_failed = True
            _increment_stat(stats_ref, "provider_failures")
            logger.warning("LLM segment-based batch translation failed: %s", e)

        if batch_failed and not parsed:
            return [""] * len(texts)

        # 2. 精准缺失补偿与兜底 (以多线程并发重试缺失索引)
        final_results = [None] * len(texts)
        missing_indices = []

        for idx in range(len(texts)):
            if idx in parsed and parsed[idx]:
                final_results[idx] = parsed[idx]
            else:
                missing_indices.append(idx)

        remaining = _seconds_left(budget_deadline, LLM_SINGLE_TIMEOUT_SECONDS)
        if missing_indices and remaining >= 1.0:
            logger.warning(
                "[LLM Segment Batch] 检测到 %d 个片段翻译缺失，正在进行精准多线程并发补偿...",
                len(missing_indices)
            )
            fallback_started = time.perf_counter()
            with ThreadPoolExecutor(max_workers=min(LLM_FALLBACK_MAX_WORKERS, len(missing_indices))) as executor:
                futures = {
                    idx: executor.submit(self._translate_one, texts[idx], source_lang, target_lang, remaining)
                    for idx in missing_indices
                }
                for idx, fut in futures.items():
                    try:
                        final_results[idx] = fut.result()
                    except Exception as fe:
                        logger.error(f"[LLM Precision Fallback] 补偿翻译索引 {idx} 失败: {fe}")
                        final_results[idx] = ""

            _increment_stat(stats_ref, "provider_fallbacks", len(missing_indices))
            _increment_stat(stats_ref, "provider_fallback_ms", int((time.perf_counter() - fallback_started) * 1000))
        elif missing_indices:
            _increment_stat(stats_ref, "provider_failures", len(missing_indices))
            for idx in missing_indices:
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
        res = self.session.get(url, timeout=BAIDU_SINGLE_TIMEOUT_SECONDS)
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
            batch_started = time.perf_counter()
            res = self.session.post(url, data=data, timeout=BAIDU_BATCH_TIMEOUT_SECONDS)
            _increment_stat(stats_ref, "provider_batch_ms", int((time.perf_counter() - batch_started) * 1000))
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


class DeepLTranslator(BaseTranslator):
    def __init__(self, api_key: str, endpoint: str = "https://api-free.deepl.com", formality: str = "default"):
        super().__init__()
        self.api_key = api_key
        self.endpoint = normalize_public_base_url(endpoint or "https://api-free.deepl.com")
        self.formality = (formality or "default").strip().lower()

    def cache_namespace(self) -> str:
        parsed = urllib.parse.urlparse(self.endpoint)
        host = parsed.hostname or self.endpoint
        return f"deepl:{host}:{self.formality}"

    def _target_lang_code(self, target_lang: str) -> str:
        mapping = {
            "zh": "ZH-HANS",
            "zh-CN": "ZH-HANS",
            "zh-TW": "ZH-HANT",
            "en": "EN-US",
            "ja": "JA",
            "ko": "KO",
            "fr": "FR",
            "de": "DE",
            "es": "ES",
            "pt": "PT-PT",
            "it": "IT",
            "ru": "RU",
            "ar": "AR",
            "tr": "TR",
        }
        return mapping.get(target_lang, (target_lang or "ZH-HANS").upper())

    def _source_lang_code(self, source_lang: str) -> str | None:
        if not source_lang or source_lang == "auto":
            return None
        mapping = {
            "zh": "ZH",
            "zh-CN": "ZH",
            "zh-TW": "ZH",
            "en": "EN",
            "ja": "JA",
            "ko": "KO",
            "fr": "FR",
            "de": "DE",
            "es": "ES",
            "pt": "PT",
            "it": "IT",
            "ru": "RU",
            "ar": "AR",
            "tr": "TR",
        }
        return mapping.get(source_lang, source_lang.upper())

    def _payload_pairs(self, texts: list[str], source_lang: str, target_lang: str) -> list[tuple[str, str]]:
        pairs = [("text", text) for text in texts]
        source_code = self._source_lang_code(source_lang)
        if source_code:
            pairs.append(("source_lang", source_code))
        target_code = self._target_lang_code(target_lang)
        pairs.append(("target_lang", target_code))
        if self.formality in {"default", "more", "less", "prefer_more", "prefer_less"}:
            if target_code in {"DE", "FR", "IT", "ES", "NL", "PL", "PT-BR", "PT-PT", "JA", "RU"}:
                pairs.append(("formality", self.formality))
        return pairs

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        result = self._do_translate_batch([text], source_lang, target_lang)
        return result[0] if result else ""

    def _do_translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []
        headers = {"Authorization": f"DeepL-Auth-Key {self.api_key}"}
        res = request_public_url(
            self.session,
            "POST",
            f"{self.endpoint}/v2/translate",
            headers=headers,
            data=self._payload_pairs(texts, source_lang, target_lang),
            timeout=DEEPL_BATCH_TIMEOUT_SECONDS,
        )
        if res.status_code != 200:
            raise Exception(f"DeepL translation failed: status {res.status_code} {res.text}")
        items = res.json().get("translations", [])
        if len(items) != len(texts):
            raise Exception(f"DeepL returned {len(items)} translations for {len(texts)} texts")
        return [str(item.get("text", "")) for item in items]

