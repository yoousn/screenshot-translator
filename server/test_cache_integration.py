# server/test_cache_integration.py
import time
from server.translator import GoogleTranslator

def test_cache_integration():
    translator = GoogleTranslator()
    
    # 模拟第一次翻译请求 (未命中缓存)
    stats1 = {"cache_hits": 0}
    texts = ["Hello my dear friend", "Goodbye for now"]
    
    # 第一次翻译，调用底层的批量翻译
    res1 = translator.translate_batch(texts, "en", "zh", stats1)
    print("Res1:", res1)
    print("Stats1:", stats1)
    
    assert stats1["cache_hits"] == 0
    
    # 第二次相同的翻译请求 (应该 100% 命中缓存)
    stats2 = {"cache_hits": 0}
    res2 = translator.translate_batch(texts, "en", "zh", stats2)
    print("Res2:", res2)
    print("Stats2:", stats2)
    
    assert res2 == res1
    assert stats2["cache_hits"] == 2 # 两行全部命中缓存!
    
    print("GoogleTranslator cache integration tests passed!")

if __name__ == "__main__":
    test_cache_integration()
