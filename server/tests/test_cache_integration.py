import pytest
import requests
from server.translator import GoogleTranslator


def test_cache_integration():
    translator = GoogleTranslator()
    stats1 = {"cache_hits": 0}
    texts = ["Hello my dear friend", "Goodbye for now"]

    try:
        res1 = translator.translate_batch(texts, "en", "zh", stats1)
    except requests.RequestException as exc:
        pytest.skip(f"Google Translate is not reachable in current env: {exc}")

    assert stats1["cache_hits"] == 0

    stats2 = {"cache_hits": 0}
    res2 = translator.translate_batch(texts, "en", "zh", stats2)

    assert res2 == res1
    assert stats2["cache_hits"] == 2
