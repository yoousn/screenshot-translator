import pytest
import numpy as np
from PIL import Image
from image_processor import ImageProcessor

def test_color_sampling():
    # 创建一个 100x100 的纯白图片，中间有一个 20x20 的黑色块
    img_arr = np.ones((100, 100, 3), dtype=np.uint8) * 255
    img_arr[40:60, 40:60] = 0 # 黑色框
    
    processor = ImageProcessor(load_ocr=False)
    bg_color = processor._sample_bg(img_arr, 40, 40, 60, 60)
    assert bg_color == (255, 255, 255)

def test_foreground_sampling():
    processor = ImageProcessor(load_ocr=False)
    bg_color = (255, 255, 255)
    fg_color = processor._choose_text_color(bg_color)
    assert fg_color == (20, 20, 20)
