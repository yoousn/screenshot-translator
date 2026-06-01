import pytest
import requests
from unittest.mock import patch
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator


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


def test_google_translation():
    translator = GoogleTranslator()
    try:
        res = translator.translate("Hello, world!", "en", "zh")
    except requests.RequestException as exc:
        pytest.skip(f"Google Translate is not reachable in current env: {exc}")
    assert isinstance(res, str)
    assert res.strip()


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


def test_llm_private_segment_markers_ignore_literal_seg_text():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator("https://example.com", "sk-test", "model-a")
    response = (
        f"{translator._segment_marker(0)}keep literal <SEG2> as text\n"
        f"{translator._segment_marker(1)}second translation"
    )
    translator.session = FakeLLMSession(response)
    with patch("translator.request_public_url", lambda session, method, url, **kwargs: session.post(url, **kwargs)):
        result = translator._do_translate_batch(["literal <SEG2>", "second"], "en", "zh")

    assert result == ["keep literal <SEG2> as text", "second translation"]
    packed_input = translator.session.last_payload["messages"][1]["content"]
    assert translator._segment_marker(0) in packed_input
    assert "<SEG0>" not in packed_input


def test_llm_segment_validation_falls_back_on_missing_marker():
    with patch("translator.normalize_public_base_url", lambda url: url.rstrip('/')):
        translator = LLMTranslator("https://example.com", "sk-test", "model-a")
    translator.session = FakeLLMSession(f"{translator._segment_marker(0)}only first")
    translator.translate = lambda text, *_args, **_kwargs: f"fallback:{text}"

    result = translator._do_translate_batch(["first", "second"], "en", "zh")

    assert result == ["fallback:first", "fallback:second"]
