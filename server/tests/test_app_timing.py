# server/test_app_timing.py
from fastapi.testclient import TestClient
import sys
import os
import io
import time
from PIL import Image, ImageDraw, ImageFont

# 将当前目录加入 Python 路径
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from server.app import app

client = TestClient(app)

def test_api_headers():
    # 1. 自适应寻找完美间隔，以 100% 触发 VirtualBlock 合并
    font_path = "C:\\Windows\\Fonts\\arial.ttf"
    if not os.path.exists(font_path):
        font_path = "C:\\Windows\\Fonts\\msyh.ttc"
    
    font_size = 32
    try:
        font = ImageFont.truetype(font_path, font_size)
        print(f"[TestImage] 成功加载字体: {font_path}, 字号: {font_size}")
    except Exception:
        font = ImageFont.load_default()
        print("[TestImage] 使用默认字体进行渲染")

    success = False
    img_bytes = None
    ocr_blocks1 = 0
    translate_units1 = 0
    translate_ms1 = 0.0
    headers1 = {}

    # 尝试不同的水平坐标间隔，找到能让 PaddleOCR 识别为 2 个框，但我们合并为 1 个翻译单元的黄金间隔
    for gap in [100, 115, 130, 145, 160, 175, 190]:
        print(f"\n--- 尝试使用坐标间隔: Hello @ 20, World @ {20 + gap} ---")
        img = Image.new("RGB", (500, 120), color="white")
        draw = ImageDraw.Draw(img)
        draw.text((20, 40), "Hello", fill="black", font=font)
        draw.text((20 + gap, 40), "World", fill="black", font=font)
        
        img_byte_arr = io.BytesIO()
        img.save(img_byte_arr, format='PNG')
        curr_bytes = img_byte_arr.getvalue()
        
        response = client.post(
            "/api/translate",
            files={"image": ("test.png", curr_bytes, "image/png")},
            headers={"x-api-key": "ysn-screenshot-translator-token-666"}
        )
        
        assert response.status_code == 200
        headers = {k.lower(): v for k, v in response.headers.items()}
        ocr_blocks = int(headers["x-trace-ocr-blocks"])
        translate_units = int(headers["x-trace-translate-units"])
        
        print(f"检测结果 -> OCR 物理框数: {ocr_blocks}, 合并后翻译单元数: {translate_units}")
        
        if ocr_blocks == 2 and translate_units == 1:
            print("[SUCCESS] 成功找到完美几何合并区间！")
            success = True
            img_bytes = curr_bytes
            ocr_blocks1 = ocr_blocks
            translate_units1 = translate_units
            translate_ms1 = float(headers["x-trace-translate-ms"])
            headers1 = headers
            break

    if not success:
        print("\n[WARNING] 提示: 各种水平间隔未能触发完美的 2->1 合并。这可能是由具体环境下的 PaddleOCR 分词策略所致。")
        print("我们将使用最后一次的测试图像继续性能打点与缓存功能的集成验证。")
        img_bytes = curr_bytes
        ocr_blocks1 = ocr_blocks
        translate_units1 = translate_units
        translate_ms1 = float(headers["x-trace-translate-ms"])
        headers1 = headers

    # 验证 trace 字段是否存在于响应头中
    assert "x-trace-total-ms" in headers1
    assert "x-trace-init-ms" in headers1
    assert "x-trace-ocr-ms" in headers1
    assert "x-trace-translate-ms" in headers1
    assert "x-trace-render-ms" in headers1
    assert "x-trace-encode-ms" in headers1
    assert "x-trace-other-ms" in headers1
    assert "x-trace-ocr-blocks" in headers1
    assert "x-trace-translate-units" in headers1
    assert "x-trace-cache-hits" in headers1

    # 验证 OCR 识别到了字
    assert ocr_blocks1 > 0, "错误：未识别到任何文字，请确认 PaddleOCR 模型及测试图像"
    
    # 4. 发起第二次翻译请求（触发缓存，验证极速响应）
    print("\n--- 发起第二次翻译请求 (热启动 / 验证 100% 缓存命中与翻译加速) ---")
    response2 = client.post(
        "/api/translate",
        files={"image": ("test.png", img_bytes, "image/png")},
        headers={"x-api-key": "ysn-screenshot-translator-token-666"}
    )
    
    headers2 = {k.lower(): v for k, v in response2.headers.items()}
    print("第二次请求响应头:")
    for k, v in response2.headers.items():
        if k.lower().startswith("x-trace"):
            print(f"  {k}: {v}")

    assert response2.status_code == 200
    ocr_blocks2 = int(headers2["x-trace-ocr-blocks"])
    translate_units2 = int(headers2["x-trace-translate-units"])
    cache_hits2 = int(headers2["x-trace-cache-hits"])
    translate_ms2 = float(headers2["x-trace-translate-ms"])

    # 验证第二次缓存命中与加速
    assert cache_hits2 >= translate_units2, "错误：第二次未实现 100% 缓存命中"
    assert translate_ms2 <= 5.0, "错误：第二次缓存翻译耗时过高"
    assert translate_units2 < ocr_blocks2, "错误：VirtualBlock 未减少翻译单元"

    print("\n======================================================================")
    print("[SUCCESS] FLAWLESS SUCCESS: 全链路毫秒级加速性能打点与自适应合并测试完美通过！")
    print(f"  - 原始 OCR 物理框数: {ocr_blocks1}")
    print(f"  - 合并后实际翻译块: {translate_units1}")
    print(f"  - 第一次测试翻译耗时: {translate_ms1:.2f} ms")
    print(f"  - 第二次缓存翻译耗时: {translate_ms2:.2f} ms")
    print("======================================================================")

if __name__ == "__main__":
    test_api_headers()
