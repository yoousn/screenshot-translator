# server/test_batch_seg.py
import re

def pack_segments(texts: list[str]) -> str:
    packed = []
    for idx, text in enumerate(texts):
        packed.append(f"<SEG{idx}>{text}")
    return "\n".join(packed)

def parse_segments(response_text: str, expected_count: int) -> dict:
    # 正则提取 <SEG(\d+)> 后面的文本，支持跨行匹配
    pattern = re.compile(r"<SEG(\d+)>\s*(.*?)(?=\s*<SEG\d+>|$)", re.DOTALL)
    matches = pattern.findall(response_text)
    
    parsed = {}
    for idx_str, content in matches:
        idx = int(idx_str)
        parsed[idx] = content.strip()
    return parsed

def test_seg_packing_and_parsing():
    # 测试 1: 完美解析
    texts = ["Hello", "World", "This is amazing"]
    packed = pack_segments(texts)
    assert packed == "<SEG0>Hello\n<SEG1>World\n<SEG2>This is amazing"
    
    response = "<SEG0> 你好\n<SEG1> 世界\n<SEG2> 这是一个神奇的旅程"
    parsed = parse_segments(response, len(texts))
    assert parsed[0] == "你好"
    assert parsed[1] == "世界"
    assert parsed[2] == "这是一个神奇的旅程"
    
    # 测试 2: 含有跨行、额外空格的解析
    response_with_newlines = """
    <SEG0>
    你好世界。
    欢迎使用。
    <SEG1>测试二
    <SEG2>   测试三   
    """
    parsed_complex = parse_segments(response_with_newlines, len(texts))
    assert parsed_complex[0] == "你好世界。\n    欢迎使用。"
    assert parsed_complex[1] == "测试二"
    assert parsed_complex[2] == "测试三"
    
    # 测试 3: 模拟 LLM 吞标记并进行精准兜底补偿
    texts = ["apple", "banana", "cherry", "date"]
    # 假设 LLM 返回结果漏掉了 banana (SEG1)
    llm_output = "<SEG0>苹果\n<SEG2>樱桃\n<SEG3>红枣"
    parsed = parse_segments(llm_output, len(texts))
    
    final_results = [None] * len(texts)
    for idx in range(len(texts)):
        if idx in parsed:
            final_results[idx] = parsed[idx]
        else:
            # 精准兜底：仅对缺失的索引单独翻译
            print(f"Fallback for missing index {idx}: {texts[idx]}")
            # 模拟单个翻译
            final_results[idx] = f"[FALLBACK]{texts[idx]}"
            
    assert final_results == ["苹果", "[FALLBACK]banana", "樱桃", "红枣"]
    print("All Segment packing, parsing and fallback tests passed!")

if __name__ == "__main__":
    test_seg_packing_and_parsing()
