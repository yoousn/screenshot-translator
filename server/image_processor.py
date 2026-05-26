import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from paddleocr import PaddleOCR
import os

class ImageProcessor:
    def __init__(self, load_ocr: bool = True):
        # 启动时加载 PaddleOCR 模型，可选以便加速测试
        if load_ocr:
            self.ocr = PaddleOCR(lang="ch")
        else:
            self.ocr = None

    def sample_background(self, img_cv, bbox):
        # bbox 格式: [x_min, y_min, x_max, y_max]
        x1, y1, x2, y2 = map(int, bbox)
        h, w = img_cv.shape[:2]
        
        # 边界向外扩 2 像素构建环形采样区
        pad = 2
        sampled_pixels = []
        for y in range(max(0, y1 - pad), min(h, y2 + pad)):
            for x in range(max(0, x1 - pad), min(w, x2 + pad)):
                # 仅采样真正的环形边缘像素，排除框内像素
                if y < y1 or y > y2 or x < x1 or x > x2:
                    sampled_pixels.append(img_cv[y, x])
                    
        if len(sampled_pixels) == 0:
            return (255, 255, 255)
        # 求中位数颜色以过滤文字边缘噪点
        median = np.median(sampled_pixels, axis=0)
        return (int(median[0]), int(median[1]), int(median[2]))

    def sample_foreground(self, img_cv, bbox, bg_color):
        x1, y1, x2, y2 = map(int, bbox)
        sub_img = img_cv[y1:y2, x1:x2]
        if sub_img.size == 0:
            return (0, 0, 0)
        
        # 统计区域内的像素与背景色差异最大的点作为文字前景色
        diff = np.linalg.norm(sub_img.astype(float) - np.array(bg_color).astype(float), axis=2)
        idx = np.unravel_index(np.argmax(diff), diff.shape)
        fg = sub_img[idx[0], idx[1]]
        return (int(fg[0]), int(fg[1]), int(fg[2]))

    def process_and_draw(self, img_bytes, translator_fn) -> bytes:
        # 从内存加载图片
        nparr = np.frombuffer(img_bytes, np.uint8)
        img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
        
        # 如果 self.ocr 还没被加载，则动态加载它（懒加载）
        if self.ocr is None:
            self.ocr = PaddleOCR(lang="ch")
            
        # 1. OCR 提取文字与区域
        ocr_result = self.ocr.ocr(img_cv, cls=True)
        if not ocr_result or not ocr_result[0]:
            return img_bytes # 无文字，直接返回原图
            
        pil_img = Image.fromarray(cv2.cvtColor(img_cv, cv2.COLOR_BGR2RGB))
        draw = ImageDraw.Draw(pil_img)
        
        # 加载中文字体，若无则使用系统默认或打包的开源字体
        font_paths = [
            "C:\\Windows\\Fonts\\msyh.ttc", # 微软雅黑
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", # Linux 备用
            "arial.ttf"
        ]
        active_font_path = next((p for p in font_paths if os.path.exists(p)), None)

        for line in ocr_result[0]:
            box = line[0] # 四角坐标 [[x1, y1], [x2, y1], [x2, y2], [x1, y2]]
            original_text = line[1][0]
            
            x_min = int(min(pt[0] for pt in box))
            x_max = int(max(pt[0] for pt in box))
            y_min = int(min(pt[1] for pt in box))
            y_max = int(max(pt[1] for pt in box))
            bbox = [x_min, y_min, x_max, y_max]
            
            # 2. 采样颜色
            bg_color = self.sample_background(img_cv, bbox)
            fg_color = self.sample_foreground(img_cv, bbox, bg_color)
            
            # 3. 擦除原文字（绘制背景色实心矩形）
            # OpenCV BGR格式颜色
            cv2.rectangle(img_cv, (x_min, y_min), (x_max, y_max), bg_color, -1)
            
            # 4. 翻译文字
            try:
                translated_text = translator_fn(original_text)
            except Exception:
                translated_text = original_text # 降级为原图原文

            # 5. 重绘文字到 Pillow Image
            # 在 Pillow 中，由于我们需要和 OpenCV 同步，这里把擦除后的背景色同步更新到 PIL 图片中
            draw.rectangle([x_min, y_min, x_max, y_max], fill=bg_color)
            
            w_box = x_max - x_min
            h_box = y_max - y_min
            
            # 寻找合适字号和自动折行
            font_size = max(8, h_box)
            font = ImageFont.truetype(active_font_path, font_size) if active_font_path else ImageFont.load_default()
            
            # 动态折行和等比缩放计算
            while font_size > 8:
                lines = []
                words = list(translated_text)
                current_line = ""
                for char in words:
                    test_line = current_line + char
                    # 计算单行文字宽度
                    text_w = draw.textlength(test_line, font=font)
                    if text_w <= w_box:
                        current_line = test_line
                    else:
                        if current_line:
                            lines.append(current_line)
                        current_line = char
                if current_line:
                    lines.append(current_line)
                
                # 计算折行后的总高度
                total_h = len(lines) * font_size
                if total_h <= h_box:
                    break
                # 高度依然超限，缩小字号并重试
                font_size -= 2
                font = ImageFont.truetype(active_font_path, font_size) if active_font_path else ImageFont.load_default()
            
            # 如果缩到最小还是太宽，我们只能强制折行
            if font_size <= 8:
                lines = []
                words = list(translated_text)
                current_line = ""
                for char in words:
                    test_line = current_line + char
                    text_w = draw.textlength(test_line, font=font)
                    if text_w <= w_box:
                        current_line = test_line
                    else:
                        if current_line:
                            lines.append(current_line)
                        current_line = char
                if current_line:
                    lines.append(current_line)
            
            # 居中重绘文字
            y_offset = y_min + (h_box - len(lines) * font_size) // 2
            for l in lines:
                text_w = draw.textlength(l, font=font)
                x_offset = x_min + (w_box - text_w) // 2
                # PIL Draw以 RGB 颜色渲染，BGR的 fg_color 需要转换成 RGB
                rgb_fg = (fg_color[2], fg_color[1], fg_color[0])
                draw.text((x_offset, y_offset), l, fill=rgb_fg, font=font)
                y_offset += font_size

        # 将处理完的 PIL 图像导出为 PNG 二进制流
        import io
        img_out = io.BytesIO()
        pil_img.save(img_out, format="PNG")
        return img_out.getvalue()
