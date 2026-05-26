import pytest
import numpy as np
from PIL import Image
from image_processor import ImageProcessor

def test_color_sampling():
    # 创建一个 100x100 的纯白图片，中间有一个 20x20 的黑色块
    img_arr = np.ones((100, 100, 3), dtype=np.uint8) * 255
    img_arr[40:60, 40:60] = 0 # 黑色框
    img = Image.fromarray(img_arr)
    
    # 采样 40,40 到 60,60 的外围背景色，预期应该识别到白色 (255, 255, 255)
    # 因为 PaddleOCR 初始化较慢，我们在测试中仅验证颜色采样这部分非神经网络的图像逻辑，
    # 或者用 unittest.mock 模拟 OCR 识别过程
    processor = ImageProcessor(load_ocr=False)
    bg_color = processor.sample_background(img_arr, [40, 40, 60, 60])
    assert bg_color == (255, 255, 255)

def test_foreground_sampling():
    # 创建一个 100x100 的纯白图片，中间有一个 20x20 的红色块
    img_arr = np.ones((100, 100, 3), dtype=np.uint8) * 255
    # 红色在 BGR 中是 (0, 0, 255)
    img_arr[40:60, 40:60] = [0, 0, 255]
    
    processor = ImageProcessor(load_ocr=False)
    bg_color = (255, 255, 255)
    fg_color = processor.sample_foreground(img_arr, [40, 40, 60, 60], bg_color)
    assert fg_color == (0, 0, 255)
