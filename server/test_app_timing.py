# server/test_app_timing.py
from fastapi.testclient import TestClient
import sys
import os

# 将当前目录加入 Python 路径
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from server.app import app

client = TestClient(app)

def test_api_headers():
    # 创建一个极小的 1x1 像素白色 PNG 图片进行测试
    import io
    from PIL import Image
    img = Image.new("RGB", (100, 100), color="white")
    img_byte_arr = io.BytesIO()
    img.save(img_byte_arr, format='PNG')
    img_bytes = img_byte_arr.getvalue()

    # 模拟请求 /api/translate
    response = client.post(
        "/api/translate",
        files={"image": ("test.png", img_bytes, "image/png")},
        headers={"x-api-key": "ysn-screenshot-translator-token-666"}
    )
    
    print("Response status code:", response.status_code)
    print("Headers:", dict(response.headers))
    
    assert response.status_code == 200
    assert "x-trace-total-ms" in response.headers
    assert "x-trace-ocr-ms" in response.headers
    assert "x-trace-translate-ms" in response.headers
    assert "x-trace-render-ms" in response.headers
    assert "x-trace-ocr-blocks" in response.headers
    assert "x-trace-translate-units" in response.headers
    assert "x-trace-cache-hits" in response.headers
    
    print("FastAPI Header Integration Verification Passed!")

if __name__ == "__main__":
    test_api_headers()
