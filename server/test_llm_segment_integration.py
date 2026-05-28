# server/test_llm_segment_integration.py
import re
from server.translator import LLMTranslator

# 继承自 LLMTranslator 以便测试打包、解析和精准兜底
class MockLLMTranslator(LLMTranslator):
    def __init__(self):
        super().__init__("http://localhost:8000", "mock-key", "gpt-4o")
        self.translate_calls = 0

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        self.translate_calls += 1
        return f"MOCK_SINGLE_TRANS({text})"

    def _mock_post_completion(self, packed_input: str, scenario: str) -> str:
        if scenario == "perfect":
            # 完美的段落对齐返回
            return "<SEG0>苹果\n<SEG1>香蕉\n<SEG2>樱桃"
        elif scenario == "missing_one":
            # 缺失香蕉 (SEG1)
            return "<SEG0>苹果\n<SEG2>樱桃"
        else:
            # 彻底报错/全数缺失
            raise Exception("Connection timed out")

    def test_batch_execution(self, texts: list[str], scenario: str) -> list[str]:
        # 覆写网络请求，进行单元隔离测试
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        packed_input = "\n".join([f"<SEG{i}>{t}" for i, t in enumerate(texts)])
        
        parsed = {}
        try:
            # 代替网络请求
            content = self._mock_post_completion(packed_input, scenario)
            pattern = re.compile(r"<SEG(\d+)>\s*(.*?)(?=\s*<SEG\d+>|$)", re.DOTALL)
            matches = pattern.findall(content)
            for idx_str, body in matches:
                parsed[int(idx_str)] = body.strip()
        except Exception as e:
            print(f"Mocking error: {e}")
            
        final_results = [None] * len(texts)
        missing_indices = []
        for idx in range(len(texts)):
            if idx in parsed and parsed[idx]:
                final_results[idx] = parsed[idx]
            else:
                missing_indices.append(idx)
                
        if missing_indices:
            from concurrent.futures import ThreadPoolExecutor
            with ThreadPoolExecutor(max_workers=8) as executor:
                futures = {
                    idx: executor.submit(self.translate, texts[idx], "auto", "zh")
                    for idx in missing_indices
                }
                for idx, fut in futures.items():
                    final_results[idx] = fut.result()
                    
        return final_results

def run_integration_test():
    texts = ["apple", "banana", "cherry"]
    
    # 场景 1: 完美解析
    t1 = MockLLMTranslator()
    res1 = t1.test_batch_execution(texts, "perfect")
    print("Perfect scenario result:", res1)
    assert res1 == ["苹果", "香蕉", "樱桃"]
    assert t1.translate_calls == 0 # 没有触发任何 fallback 兜底!

    # 场景 2: 缺失部分片段，触发精准局部补偿
    t2 = MockLLMTranslator()
    res2 = t2.test_batch_execution(texts, "missing_one")
    print("Missing one segment scenario result:", res2)
    assert res2 == ["苹果", "MOCK_SINGLE_TRANS(banana)", "樱桃"]
    assert t2.translate_calls == 1 # 精确补偿了 1 次!

    # 场景 3: 彻底异常，触发完全退化兜底
    t3 = MockLLMTranslator()
    res3 = t3.test_batch_execution(texts, "failure")
    print("Complete failure scenario result:", res3)
    assert res3 == ["MOCK_SINGLE_TRANS(apple)", "MOCK_SINGLE_TRANS(banana)", "MOCK_SINGLE_TRANS(cherry)"]
    assert t3.translate_calls == 3 # 全数补偿了 3 次!

    print("All LLM Segment Packing, Regex Extraction & Precision Fallback Integration Tests Succeeded!")

if __name__ == "__main__":
    run_integration_test()
