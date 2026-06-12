import pytest
import requests
from unittest.mock import patch
from translator import BaseTranslator, GoogleTranslator, LLMTranslator, BaiduTranslator, DeepLTranslator, SEGMENT_SEPARATOR
from http_client import get_official_translation_session, get_public_session


class FakeLLMResponse:
    status_code = 200

    def __init__(self, content: str):
        self._content = content

    def json(self):
        return {"choices": [{"message": {"content": self._content}}]}


class FakeLLMSession:
    def __init__(self, content: str):
        self.content = content
        self.last_payload = None

    def post(self, _url, **kwargs):
        self.last_payload = kwargs.get("json")
        return FakeLLMResponse(self.content)


class FakeDeepLResponse:
    status_code = 200
    text = "ok"

    def json(self):
        return {"translations": [{"text": "你好"}, {"text": "世界"}]}


class FakeDeepLSession:
    def __init__(self):
        self.last_headers = None
        self.last_data = None

    def request(self, _method, _url, **kwargs):
        self.last_headers = kwargs.get("headers")
        self.last_data = kwargs.get("data")
        return FakeDeepLResponse()


class FakeGoogleBatchResponse:
    status_code = 200

    def __init__(self, translated_full: str):
        self.translated_full = translated_full

    def json(self):
        return [[[self.translated_full]]]


class FakeGoogleBatchSession:
    def __init__(self, translated_full: str):
        self.translated_full = translated_full
        self.last_data = None

    def post(self, _url, data, timeout, **_kwargs):
        self.last_data = data
        return FakeGoogleBatchResponse(self.translated_full)


def test_google_translation():
    translator = GoogleTranslator()
    try:
        res = translator.translate("Hello, world!", "en", "zh")
    except requests.RequestException as exc:
        pytest.skip(f"Google Translate is not reachable in current env: {exc}")
    assert isinstance(res, str)
    assert res.strip()


def test_official_providers_use_proxy_compatible_session():
    assert GoogleTranslator().session is get_official_translation_session()
    assert BaiduTranslator("app-id", "secret").session is get_official_translation_session()
    assert DeepLTranslator("deepl-key").session is get_official_translation_session()
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip("/")):
        llm = LLMTranslator("https://example.com", "sk-test", "model-a")
    assert llm.session is get_public_session()
    assert get_public_session() is not get_official_translation_session()


def test_google_batch_uses_stable_segment_separator():
    translator = GoogleTranslator()
    translator.session = FakeGoogleBatchSession(f"打开{SEGMENT_SEPARATOR}保存")

    result = translator._do_translate_batch(["Open", "Save"], "en", "zh", {})

    assert result == ["打开", "保存"]
    assert SEGMENT_SEPARATOR in translator.session.last_data["q"]


def test_llm_translation_format():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator(
            base_url="https://example.com",
            api_key="sk-test",
            model="gemini-1.5-flash"
        )
    try:
        res = translator.translate("Hello", "en", "zh")
        assert isinstance(res, str)
    except Exception as e:
        pytest.skip(f"new-api not reachable in current env: {e}")


def test_llm_rejects_private_base_url():
    with pytest.raises(ValueError):
        LLMTranslator(
            base_url="http://127.0.0.1:3001",
            api_key="sk-test",
            model="gemini-1.5-flash"
        )


def test_llm_cache_namespace_includes_model():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator_a = LLMTranslator("https://example.com", "sk-test", "model-a")
        translator_b = LLMTranslator("https://example.com", "sk-test", "model-b")
    assert translator_a.cache_namespace() != translator_b.cache_namespace()


def test_llm_prompt_replaces_language_and_domain_placeholders():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator(
            "https://example.com",
            "sk-test",
            "model-a",
            "Translate {{SOURCE_LANGUAGE}} to {{TARGET_LANGUAGE}} for {{TRANSLATION_DOMAIN}}.",
            "screenshot UI",
        )
    translator.session = FakeLLMSession("你好")
    with patch("translator.request_public_url", lambda session, method, url, **kwargs: session.post(url, **kwargs)):
        translator.translate("Hello", "en", "zh")

    system_prompt = translator.session.last_payload["messages"][0]["content"]
    assert "English" in system_prompt
    assert "Simplified Chinese" in system_prompt
    assert "screenshot UI" in system_prompt


def test_llm_percent_segments_ignore_literal_seg_text():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator("https://example.com", "sk-test", "model-a")
    response = "keep literal <SEG2> as text\n%%\nsecond translation"
    translator.session = FakeLLMSession(response)
    with patch("translator.request_public_url", lambda session, method, url, **kwargs: session.post(url, **kwargs)):
        result = translator._do_translate_batch(["literal <SEG2>", "second"], "en", "zh")

    assert result == ["keep literal <SEG2> as text", "second translation"]
    packed_input = translator.session.last_payload["messages"][1]["content"]
    assert "\n%%\n" in packed_input
    assert "<SEG0>" not in packed_input


def test_llm_percent_segment_validation_falls_back_on_missing_separator():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator("https://example.com", "sk-test", "model-a")
    translator.session = FakeLLMSession("only first")
    translator._translate_one = lambda text, *_args, **_kwargs: f"fallback:{text}"

    with patch("translator.request_public_url", lambda session, method, url, **kwargs: session.post(url, **kwargs)):
        result = translator._do_translate_batch(["first", "second"], "en", "zh")

    assert result == ["fallback:first", "fallback:second"]


def test_llm_batch_network_failure_does_not_start_slow_single_fallback():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator("https://example.com", "sk-test", "model-a")
    fallback_calls = []
    translator._translate_one = lambda text, *_args, **_kwargs: fallback_calls.append(text) or f"fallback:{text}"

    with patch("translator.request_public_url", side_effect=requests.Timeout("batch timeout")):
        result = translator._do_translate_batch(["first", "second"], "en", "zh")

    assert result == ["", ""]
    assert fallback_calls == []


def test_deepl_batch_uses_official_v2_translate_protocol():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = DeepLTranslator("deepl-key", "https://api-free.deepl.com", "prefer_more")
    translator.session = FakeDeepLSession()
    with patch("translator.request_public_url", lambda session, method, url, **kwargs: session.request(method, url, **kwargs)):
        result = translator._do_translate_batch(["Hello", "World"], "en", "zh")

    assert result == ["你好", "世界"]
    assert translator.session.last_headers["Authorization"] == "DeepL-Auth-Key deepl-key"
    assert ("target_lang", "ZH-HANS") in translator.session.last_data
    assert ("source_lang", "EN") in translator.session.last_data
    assert translator.session.last_data.count(("text", "Hello")) == 1


def test_deepl_rejects_non_official_endpoint():
    with pytest.raises(ValueError):
        DeepLTranslator("deepl-key", "https://127.0.0.1:8318")


class CountingTranslator(BaseTranslator):
    def __init__(self):
        super().__init__()
        self.provider_batches = []

    def cache_namespace(self) -> str:
        return f"test-dedupe:{id(self)}"

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        return f"translated:{text}"

    def _do_translate_batch(self, texts, source_lang, target_lang, stats_ref=None):
        self.provider_batches.append(list(texts))
        return [f"translated:{text}" for text in texts]


def test_base_translator_deduplicates_same_request_misses():
    translator = CountingTranslator()
    stats = {"cache_hits": 0}

    result = translator.translate_batch(
        ["Repeat me", "Repeat me", "Unique text", "Repeat me"],
        "en",
        "zh",
        stats,
    )

    assert result == [
        "translated:Repeat me",
        "translated:Repeat me",
        "translated:Unique text",
        "translated:Repeat me",
    ]
    assert translator.provider_batches == [["Repeat me", "Unique text"]]
    assert stats["cache_hits"] == 0
    assert stats["provider_misses"] == 2
    assert stats["request_duplicates"] == 2


def test_base_translator_preserves_technical_text_without_provider():
    translator = CountingTranslator()
    stats = {}

    result = translator.translate_batch(
        [
            "COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md",
            "PATH=C:\\Windows\\System32 && LocalModel.exe --help",
            "Translate me",
        ],
        "en",
        "zh",
        stats,
    )

    assert result == [
        "COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md",
        "PATH=C:\\Windows\\System32 && LocalModel.exe --help",
        "translated:Translate me",
    ]
    assert translator.provider_batches == [["Translate me"]]
    assert stats["preserved_hits"] == 2
    assert stats["provider_misses"] == 1


def test_base_translator_does_not_preserve_japanese_kanji_kana_text():
    translator = CountingTranslator()
    stats = {}

    result = translator.translate_batch(["保存する前にプレビューを開く"], "ja", "zh", stats)

    assert result == ["translated:保存する前にプレビューを開く"]
    assert translator.provider_batches == [["保存する前にプレビューを開く"]]
    assert stats["preserved_hits"] == 0
    assert stats["provider_misses"] == 1
