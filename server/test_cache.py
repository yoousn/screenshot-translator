# server/test_cache.py
import time
import threading

# 简单的线程安全 LRU + TTL 缓存实现，用于验证测试
class TranslationCache:
    def __init__(self, maxsize=5000, ttl_seconds=86400):
        self.maxsize = maxsize
        self.ttl_seconds = ttl_seconds
        self.lock = threading.RLock()
        self.cache = {} # key -> (value, expire_time)
        self.access_order = [] # key 访问顺序，最旧的在前面

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
                # 已过期
                self.cache.pop(key, None)
                if key in self.access_order:
                    self.access_order.remove(key)
                return None
            # 刷新访问顺序
            if key in self.access_order:
                self.access_order.remove(key)
            self.access_order.append(key)
            return val

    def set(self, key: tuple, value: str):
        with self.lock:
            # 驱逐旧元素
            if len(self.cache) >= self.maxsize and key not in self.cache:
                oldest = self.access_order.pop(0)
                self.cache.pop(oldest, None)
            
            expire = time.time() + self.ttl_seconds
            self.cache[key] = (value, expire)
            if key in self.access_order:
                self.access_order.remove(key)
            self.access_order.append(key)

def test_cache_lru_and_ttl():
    # 测试 1: 基本缓存存取
    cache = TranslationCache(maxsize=3, ttl_seconds=0.5)
    key1 = cache.make_key(" Hello   World \n", "en", "zh", "google", "1.0")
    key1_alt = cache.make_key("Hello World", "en", "zh", "google", "1.0")
    assert key1 == key1_alt # 保证折叠空白后的 key 相同
    
    cache.set(key1, "你好世界")
    assert cache.get(key1_alt) == "你好世界"
    
    # 测试 2: TTL 过期
    time.sleep(0.6)
    assert cache.get(key1) is None
    
    # 测试 3: LRU 驱逐
    cache.set(cache.make_key("one", "en", "zh", "google", "1.0"), "1")
    cache.set(cache.make_key("two", "en", "zh", "google", "1.0"), "2")
    cache.set(cache.make_key("three", "en", "zh", "google", "1.0"), "3")
    
    # 访问 "one" 使其变新
    cache.get(cache.make_key("one", "en", "zh", "google", "1.0"))
    
    # 添加第四个元素，这应该驱逐 "two" (因为 "one" 刚被访问，而 "three" 较新)
    cache.set(cache.make_key("four", "en", "zh", "google", "1.0"), "4")
    
    assert cache.get(cache.make_key("two", "en", "zh", "google", "1.0")) is None
    assert cache.get(cache.make_key("one", "en", "zh", "google", "1.0")) == "1"
    assert cache.get(cache.make_key("three", "en", "zh", "google", "1.0")) == "3"
    assert cache.get(cache.make_key("four", "en", "zh", "google", "1.0")) == "4"
    
    print("All TranslationCache unit tests passed!")

if __name__ == "__main__":
    test_cache_lru_and_ttl()
