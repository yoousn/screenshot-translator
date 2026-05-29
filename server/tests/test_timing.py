# server/test_timing.py
import time

def simulate_timing():
    start = time.perf_counter()
    time.sleep(0.05) # 模拟 OCR
    ocr_done = time.perf_counter()
    time.sleep(0.08) # 模拟 翻译
    trans_done = time.perf_counter()
    time.sleep(0.03) # 模拟 渲染
    render_done = time.perf_counter()
    
    t_ocr = (ocr_done - start) * 1000
    t_trans = (trans_done - ocr_done) * 1000
    t_render = (render_done - trans_done) * 1000
    t_total = (render_done - start) * 1000
    
    print(f"OCR: {t_ocr:.2f}ms, Trans: {t_trans:.2f}ms, Render: {t_render:.2f}ms, Total: {t_total:.2f}ms")
    assert abs(t_total - (t_ocr + t_trans + t_render)) < 0.1
    assert t_ocr > 40
    assert t_trans > 70
    assert t_render > 20
    print("Timing calculation test passed!")

if __name__ == "__main__":
    simulate_timing()
