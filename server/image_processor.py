import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from paddleocr import PaddleOCR
import os

class ImageProcessor:
    def __init__(self, load_ocr: bool = True):
        # 启动时加载 PaddleOCR 模型，可选以加速测试。
        # 优化 PaddleOCR 内部参数：大幅降低 det_db_box_thresh 和 det_db_thresh，从而精准捕获屏幕上的微小字体、淡字与模糊字。
        if load_ocr:
            self.ocr = PaddleOCR(
                lang="ch",
                enable_mkldnn=False,
                ir_optim=False,
                det_db_box_thresh=0.3,  # 大幅降低文字框检测阈值，专门捕获超小字
                det_db_thresh=0.2,      # 降低二值化阈值，即使低对比度/模糊的小字也绝不放过
                det_db_unclip_ratio=1.6 # 增加边界未剪切比例，使小字框周围有更舒适的余量
            )
        else:
            self.ocr = None

    def sample_background(self, img_cv, bbox):
        # bbox 格式: [x_min, y_min, x_max, y_max]
        x1, y1, x2, y2 = map(int, bbox)
        h, w = img_cv.shape[:2]
        
        # 边界向外扩 3 像素构建环形采样区，保证对原背景采样更加精准
        pad = 3
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
        fg_color = (int(fg[0]), int(fg[1]), int(fg[2]))
        
        # 🚀 智能对比度提升引擎 (Contrast Booster)
        # 计算背景与前景的相对亮度 (Luminance)：Y = 0.299*R + 0.587*G + 0.114*B
        # 注意：OpenCV 读取的是 BGR 格式，fg_color 和 bg_color 为 (B, G, R)
        bg_lum = 0.299 * bg_color[2] + 0.587 * bg_color[1] + 0.114 * bg_color[0]
        fg_lum = 0.299 * fg_color[2] + 0.587 * fg_color[1] + 0.114 * fg_color[0]
        
        # 如果亮度差值太低（即文字与背景色过于接近，字迹会模糊不清），强力提升对比度
        if abs(bg_lum - fg_lum) < 70:
            if bg_lum < 128:
                # 暗色背景 -> 强制使用纯白或高亮前景，保证刺目清晰
                fg_color = (255, 255, 255)
            else:
                # 亮色背景 -> 强制使用深灰/纯黑前景
                fg_color = (30, 30, 30)
                
        return fg_color

    def process_and_draw(self, img_bytes, translator_batch_fn) -> bytes:
        try:
            # 从内存加载图片
            nparr = np.frombuffer(img_bytes, np.uint8)
            img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
            if img_cv is None:
                return img_bytes
                
            # 如果 self.ocr 还没被加载，则动态加载它（懒加载）
            if self.ocr is None:
                self.ocr = PaddleOCR(
                    lang="ch",
                    enable_mkldnn=False,
                    ir_optim=False,
                    det_db_box_thresh=0.3,
                    det_db_thresh=0.2,
                    det_db_unclip_ratio=1.6
                )
                
            # 1. OCR 提取文字与区域
            ocr_result = self.ocr.ocr(img_cv, cls=True)
            if not ocr_result or not ocr_result[0]:
                return img_bytes # 无文字，直接返回原图
                
            pil_img = Image.fromarray(cv2.cvtColor(img_cv, cv2.COLOR_BGR2RGB))
            draw = ImageDraw.Draw(pil_img)
            
            # 加载中文字体，若无则使用系统默认或打包的开源字体
            user_font_dir = os.path.expanduser("~/.screenshot-translator")
            os.makedirs(user_font_dir, exist_ok=True)
            user_font_path = os.path.join(user_font_dir, "wqy-microhei.ttc")

            font_paths = [
                user_font_path,
                "C:\\Windows\\Fonts\\msyh.ttc", # 微软雅黑
                "C:\\Windows\\Fonts\\simhei.ttf", # 黑体
                "C:\\Windows\\Fonts\\simsun.ttc", # 宋体
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", # Linux 文泉驿微米黑
                "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
                "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc", # Linux 文泉驿正黑
                "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", # Linux Droid
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", # Linux Noto
                "/usr/share/fonts/truetype/arphic/uming.ttc",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", # Linux 备用
                "arial.ttf"
            ]
            active_font_path = next((p for p in font_paths if os.path.exists(p)), None)

            # 如果没有找到任何中文字体，则自动从高速 CDN 下载文泉驿微米黑字体
            if not active_font_path or active_font_path == "arial.ttf":
                try:
                    print("[FontManager] 未在系统检测到中文字体，正在从 jsDelivr CDN 极速下载文泉驿微米黑字体...")
                    import urllib.request
                    font_url = "https://cdn.jsdelivr.net/gh/anthonyfok/fonts-wqy-microhei@master/wqy-microhei.ttc"
                    req = urllib.request.Request(font_url, headers={'User-Agent': 'Mozilla/5.0'})
                    with urllib.request.urlopen(req, timeout=15) as response, open(user_font_path, 'wb') as out_file:
                        out_file.write(response.read())
                    if os.path.exists(user_font_path) and os.path.getsize(user_font_path) > 1000000:
                        active_font_path = user_font_path
                        print("[FontManager] 字体下载并加载成功！路径:", active_font_path)
                except Exception as fe:
                    print("[FontManager] 自动下载字体失败 (可能无网络):", fe)
    
            # 收集所有需要翻译的文本
            original_texts = []
            for line in ocr_result[0]:
                original_texts.append(line[1][0])
    
            # 一次性批量翻译
            try:
                translated_texts = translator_batch_fn(original_texts)
            except Exception as te:
                print("Translation error:", te)
                translated_texts = original_texts
    
            # 保证长度一致
            if len(translated_texts) != len(original_texts):
                translated_texts = original_texts
    
            for i, line in enumerate(ocr_result[0]):
                box = line[0] # 四角坐标 [[x1, y1], [x2, y1], [x2, y2], [x1, y2]]
                translated_text = translated_texts[i]
                
                x_min = int(min(pt[0] for pt in box))
                x_max = int(max(pt[0] for pt in box))
                y_min = int(min(pt[1] for pt in box))
                y_max = int(max(pt[1] for pt in box))
                bbox = [x_min, y_min, x_max, y_max]
                
                # 2. 采样颜色
                bg_color = self.sample_background(img_cv, bbox)
                fg_color = self.sample_foreground(img_cv, bbox, bg_color)
                
                # 🚀 消除原字边缘残留 (Padding Erase)
                # 原始 OCR 识别框较窄，原文字边缘常有残存发丝细线或抗锯齿灰色边缘。
                # 通过向外扩张 2-3 像素做全区域背景擦除，保证译文绝对“干净”，无旧字重影残留。
                w_box = x_max - x_min
                h_box = y_max - y_min
                pad_x = max(2, int(w_box * 0.02))
                pad_y = max(2, int(h_box * 0.04))
                
                x_min_pad = max(0, x_min - pad_x)
                x_max_pad = min(img_cv.shape[1], x_max + pad_x)
                y_min_pad = max(0, y_min - pad_y)
                y_max_pad = min(img_cv.shape[0], y_max + pad_y)
                
                # 3. 擦除原文字（绘制背景色实心矩形）
                cv2.rectangle(img_cv, (x_min_pad, y_min_pad), (x_max_pad, y_max_pad), bg_color, -1)
                
                # 5. 重绘文字到 Pillow Image (将擦除后的背景色同步更新到 PIL 图片中)
                draw.rectangle([x_min_pad, y_min_pad, x_max_pad, y_max_pad], fill=bg_color)
                
                # 🚀 智能可读性字号限制 (Readable Minimum Font Size)
                # 即使原文字极其微小，中文翻译也必须强制至少为 11px，否则笔画交织，人眼根本无法阅读且极度模糊。
                font_size = max(11, h_box)
                font = ImageFont.truetype(active_font_path, font_size) if active_font_path else ImageFont.load_default()
                
                # 动态折行和等比缩放计算
                while font_size > 11:
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
                    
                    # 计算折行后的总高度 (中文标准行间距通常为 1.1 - 1.15)
                    line_spacing = int(font_size * 1.1)
                    total_h = (len(lines) - 1) * line_spacing + font_size if lines else 0
                    if total_h <= h_box:
                        break
                    # 高度依然超限，缩小字号并重试
                    font_size -= 1
                    font = ImageFont.truetype(active_font_path, font_size) if active_font_path else ImageFont.load_default()
                
                # 如果缩到最小可读字号 11px 还是太宽，我们只能在 11px 强行折行
                if font_size <= 11:
                    font_size = 11
                    font = ImageFont.truetype(active_font_path, font_size) if active_font_path else ImageFont.load_default()
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
                
                # 🚀 杜绝字飘，使用 Pillow Anchor 系统进行完美中线对齐 (Perfect Middle Centering)
                # 传统通过 (x_offset, y_offset) 绘制依赖文字顶部，会导致字符由于拼音或英文字形等存在上下偏移，从而产生“字飘了”的感觉。
                # 采用 Pillow 的 anchor="mm"（中线中点对齐），数学上严格令文字垂直和水平中点对齐。
                x_center = x_min + w_box / 2
                y_center = y_min + h_box / 2
                
                line_spacing = int(font_size * 1.1)
                total_h = (len(lines) - 1) * line_spacing + font_size if lines else 0
                
                # PIL Draw以 RGB 颜色渲染，BGR的 fg_color 和 bg_color 需要转换成 RGB
                rgb_fg = (fg_color[2], fg_color[1], fg_color[0])
                rgb_bg = (bg_color[2], bg_color[1], bg_color[0])
                
                for idx, l in enumerate(lines):
                    # 计算当前行中心的精确 y 轴坐标，令整个多行文本块中线完美贴合边界框中线
                    line_y = y_center - (total_h / 2) + idx * line_spacing + (font_size / 2)
                    
                    # 🚀 智能轻微描边抗锯齿 (Anti-Aliasing Stroke)
                    # 对于 >= 13px 的字号，加上 1px 与背景色相同的平滑描边，有效消除锯齿阴影，让边缘极度锐利清晰
                    if font_size >= 13:
                        draw.text(
                            (x_center, line_y), 
                            l, 
                            fill=rgb_fg, 
                            font=font, 
                            anchor="mm", 
                            stroke_width=1, 
                            stroke_fill=rgb_bg
                        )
                    else:
                        draw.text(
                            (x_center, line_y), 
                            l, 
                            fill=rgb_fg, 
                            font=font, 
                            anchor="mm"
                        )
    
            # 将处理完的 PIL 图像导出为 PNG 二进制流
            import io
            img_out = io.BytesIO()
            pil_img.save(img_out, format="PNG")
            return img_out.getvalue()
        except Exception as e:
            print("ERROR in process_and_draw:", e)
            import traceback
            traceback.print_exc()
            return img_bytes
