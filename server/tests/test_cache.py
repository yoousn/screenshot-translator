import time

from translator import TranslationCache


def test_cache_lru_and_ttl():
    cache = TranslationCache(maxsize=3, ttl_seconds=0.5)
    key1 = cache.make_key(" Hello   World \n", "en", "zh", "google", "1.0")
    key1_alt = cache.make_key("Hello World", "en", "zh", "google", "1.0")
    assert key1 == key1_alt

    cache.set(key1, "hello world")
    assert cache.get(key1_alt) == "hello world"

    time.sleep(0.6)
    assert cache.get(key1) is None

    cache.set(cache.make_key("one", "en", "zh", "google", "1.0"), "1")
    cache.set(cache.make_key("two", "en", "zh", "google", "1.0"), "2")
    cache.set(cache.make_key("three", "en", "zh", "google", "1.0"), "3")

    cache.get(cache.make_key("one", "en", "zh", "google", "1.0"))
    cache.set(cache.make_key("four", "en", "zh", "google", "1.0"), "4")

    assert cache.get(cache.make_key("two", "en", "zh", "google", "1.0")) is None
    assert cache.get(cache.make_key("one", "en", "zh", "google", "1.0")) == "1"
    assert cache.get(cache.make_key("three", "en", "zh", "google", "1.0")) == "3"
    assert cache.get(cache.make_key("four", "en", "zh", "google", "1.0")) == "4"
