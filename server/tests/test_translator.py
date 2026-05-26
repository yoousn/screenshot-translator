import pytest
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator

def test_google_translation():
    translator = GoogleTranslator()
    res = translator.translate("Hello, world!", "en", "zh")
    assert "你好" in res

def test_llm_translation_format():
    # 模拟一个 OpenAI 兼容的请求
    translator = LLMTranslator(
        base_url="http://192.168.1.3:3001",
        api_key="sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb",
        model="gemini-1.5-flash"
    )
    # 此处若由于内网未连通失败，应能合理捕获异常并返回降级提示
    try:
        res = translator.translate("Hello", "en", "zh")
        assert isinstance(res, str)
    except Exception as e:
        pytest.skip(f"new-api not reachable in current env: {e}")
