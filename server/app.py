import sys
import os
# Ensure server directory is in sys.path so imports work regardless of CWD
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from fastapi import FastAPI, UploadFile, File, Header, HTTPException, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
import requests
import cv2
import numpy as np
import urllib.parse
import socket
import ipaddress
import time
from config import load_server_config, save_server_config
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator
from image_processor import ImageProcessor

app = FastAPI(title="Screenshot Translator API")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)

# 默认不加载 heavy OCR 模型以加速服务启动，首个翻译请求来时会触发懒加载
processor = ImageProcessor(load_ocr=False)

# 🌟 后台异步加载并预热 OCR 模型以消除首次请求的冷启动卡顿
def warm_up_ocr_async():
    try:
        time.sleep(1.5)
        print("[OCR Background Warmup] Waking up PaddleOCR models silently...")
        processor._ensure_ocr()
        print("[OCR Background Warmup] PaddleOCR models are 100% warmed up and ready for hot-requests!")
    except Exception as e:
        print("[OCR Background Warmup] Warmup background thread warning:", e)

import threading
threading.Thread(target=warm_up_ocr_async, daemon=True).start()


_config_cache = None
_config_cache_time = 0.0
_CONFIG_TTL = 5.0

def get_config():
    global _config_cache, _config_cache_time
    now = time.time()
    if _config_cache is None or (now - _config_cache_time) > _CONFIG_TTL:
        _config_cache = load_server_config()
        _config_cache_time = now
    return _config_cache

def verify_token(x_api_key: str):
    cfg = get_config()
    if not x_api_key or x_api_key != cfg["client_token"]:
        raise HTTPException(status_code=401, detail="Unauthorized client token.")

def get_active_translator():
    cfg = get_config()
    channel = cfg.get("active_channel", "google")
    print(f"[Active Translator] 服务器当前激活的翻译通道为: '{channel}'")
    if channel == "new-api":
        c = cfg["channels"]["new-api"]
        print(f"[Active Translator] 正在调用大模型翻译 (中转: {c.get('base_url')}, 模型: {c.get('model')})")
        return LLMTranslator(c["base_url"], c["api_key"], c["model"])
    elif channel == "baidu":
        c = cfg["channels"]["baidu"]
        print(f"[Active Translator] 正在调用百度翻译 (AppID: {c.get('app_id')})")
        return BaiduTranslator(c["app_id"], c["secret_key"])
    print("[Active Translator] 未识别或默认通道，正在回退调用 Google 免费翻译接口 (无凭证)")
    return GoogleTranslator()

def _validate_url(url: str) -> bool:
    try:
        parsed = urllib.parse.urlparse(url)
        hostname = parsed.hostname
        if not hostname:
            return False
        addr_info = socket.getaddrinfo(hostname, None)
        for family, _, _, _, sockaddr in addr_info:
            ip_str = sockaddr[0]
            if '%' in ip_str:
                ip_str = ip_str.split('%')[0]
            ip = ipaddress.ip_address(ip_str)
            if ip.is_private or ip.is_loopback or ip.is_link_local or ip.is_reserved:
                return False
        return True
    except Exception:
        return False

@app.get("/api/health")
async def health_check():
    return {"status": "ok"}

@app.get("/c_hello")
async def c_hello(asker: str = ""):
    return {
        "status": "ok",
        "message": "hello",
        "asker": asker,
        "server": "local"
    }

@app.post("/api/translate")

def translate_image(image: UploadFile = File(...), x_api_key: str = Header(None)):
    verify_token(x_api_key)
    img_bytes = image.file.read()
    
    # 动态获取当前激活的翻译引擎
    translator = get_active_translator()
    def translator_batch_fn(texts, stats_ref):
        return translator.translate_batch(texts, "auto", "zh", stats_ref)
        
    try:
        out_bytes, stats = processor.process_and_draw(img_bytes, translator_batch_fn, config=get_config())
        
        # 终端漂亮的可视化耗时报告
        if get_config().get("debug_trace", True):
            total = max(stats["total_ms"], 0.001)
            report_lines = [
                "+--------------------------------------------------------+",
                "|               [TIMER] TRANSLATION REPORT               |",
                "+--------------------------------------------------------+",
                f"|  Total Duration:     {stats['total_ms']:8.2f} ms                     |",
                f"|  +- Init Step:       {stats.get('init_ms', 0.0):8.2f} ms  ({stats.get('init_ms', 0.0)/total*100:5.1f}%)        |",
                f"|  +- OCR Step:        {stats['ocr_ms']:8.2f} ms  ({stats['ocr_ms']/total*100:5.1f}%)        |",
                f"|  +- Translate Step:  {stats['translate_ms']:8.2f} ms  ({stats['translate_ms']/total*100:5.1f}%)        |",
                f"|  +- Render Step:     {stats['render_ms']:8.2f} ms  ({stats['render_ms']/total*100:5.1f}%)        |",
                f"|  +- Encode Step:     {stats.get('encode_ms', 0.0):8.2f} ms  ({stats.get('encode_ms', 0.0)/total*100:5.1f}%)        |",
                f"|  +- Other Step:      {stats.get('other_ms', 0.0):8.2f} ms  ({stats.get('other_ms', 0.0)/total*100:5.1f}%)        |",
                "+--------------------------------------------------------+",
                f"|  OCR Blocks:         {stats['ocr_blocks']:8d}                          |",
                f"|  Translate Units:    {stats['translate_units']:8d}                          |",
                f"|  Cache Hits:         {stats['cache_hits']:8d}                          |",
                "+--------------------------------------------------------+"
            ]
            for line in report_lines:
                try:
                    print(line)
                except UnicodeEncodeError:
                    print(line.encode('ascii', 'ignore').decode('ascii'))

        headers = {
            "X-Trace-Total-Ms": f"{stats['total_ms']:.2f}",
            "X-Trace-Init-Ms": f"{stats.get('init_ms', 0.0):.2f}",
            "X-Trace-Ocr-Ms": f"{stats['ocr_ms']:.2f}",
            "X-Trace-Translate-Ms": f"{stats['translate_ms']:.2f}",
            "X-Trace-Render-Ms": f"{stats['render_ms']:.2f}",
            "X-Trace-Encode-Ms": f"{stats.get('encode_ms', 0.0):.2f}",
            "X-Trace-Other-Ms": f"{stats.get('other_ms', 0.0):.2f}",
            "X-Trace-Ocr-Blocks": str(stats["ocr_blocks"]),
            "X-Trace-Translate-Units": str(stats["translate_units"]),
            "X-Trace-Cache-Hits": str(stats["cache_hits"]),
            "X-Ocr-Ready": "true" if stats.get("ocr_ready", False) else "false",
            "X-Ocr-Cache-Hit": "true" if stats.get("ocr_cache_hit", False) else "false"
        }
        return Response(content=out_bytes, media_type="image/png", headers=headers)
    except Exception as e:
        print(f"[translate_image] error during process_and_draw: {e}")
        raise HTTPException(status_code=500, detail=f"Image processing failed: {str(e)}")


@app.post("/api/ocr")
async def ocr_image(image: UploadFile = File(...), x_api_key: str = Header(None)):
    try:
        verify_token(x_api_key)
        img_bytes = await image.read()

        nparr = np.frombuffer(img_bytes, np.uint8)
        img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
        if img_cv is None:
            return JSONResponse(status_code=400, content={"status": "failed", "error": "图片解码失败"})

        processor._ensure_ocr()
        ocr_result = processor.ocr.ocr(img_cv, cls=True)

        results = []
        if ocr_result and ocr_result[0]:
            for line in ocr_result[0]:
                box = line[0]
                text = line[1][0]
                confidence = float(line[1][1])
                results.append({
                    "box": box,
                    "text": text,
                    "confidence": confidence
                })

        return {"status": "success", "ocr": results}
    except HTTPException:
        raise
    except Exception as e:
        return JSONResponse(status_code=500, content={"status": "failed", "error": f"OCR 处理失败: {str(e)}"})

from pydantic import BaseModel
from typing import List, Optional

class OcrBlockModel(BaseModel):
    text: str
    confidence: float
    box: List[List[float]]

class TranslateTextRequest(BaseModel):
    blocks: List[OcrBlockModel]
    source_lang: Optional[str] = "auto"
    target_lang: Optional[str] = "zh"
    render_mode: Optional[str] = "client"

@app.post("/api/translate_text")
async def translate_text_endpoint(
    req: TranslateTextRequest,
    x_api_key: str = Header(None, alias="x-api-key")
):
    verify_token(x_api_key)
    
    if not req.blocks:
        return {
            "status": "success",
            "translations": [],
            "cache_hits": 0,
            "channel": get_config().get("active_channel", "google")
        }
    
    texts = [block.text for block in req.blocks]
    
    # 动态获取当前激活的翻译引擎
    translator = get_active_translator()
    
    stats_ref = {"cache_hits": 0}
    try:
        translations = translator.translate_batch(texts, req.source_lang, req.target_lang, stats_ref)
    except Exception as e:
        print(f"[translate_text] error during batch translation: {e}")
        # 降级：如果 translate_batch 崩溃，则对单个单词独立处理，容错性极强
        translations = []
        for text in texts:
            try:
                res = translator.translate(text, source_lang=req.source_lang, target_lang=req.target_lang)
                translations.append(res)
            except Exception:
                translations.append(text)
                
    return {
        "status": "success",
        "translations": translations,
        "cache_hits": stats_ref["cache_hits"],
        "channel": get_config().get("active_channel", "google")
    }

@app.post("/api/config/test")
def test_and_save_config(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    channel = payload.get("channel")
    c = payload.get("config", {})
    
    try:
        # 1. 临时实例化对应的翻译器进行连通性验证
        if channel == "new-api":
            temp_t = LLMTranslator(c.get("base_url"), c.get("api_key"), c.get("model"))
        elif channel == "baidu":
            temp_t = BaiduTranslator(c.get("app_id"), c.get("secret_key"))
        else:
            temp_t = GoogleTranslator()
            
        test_res = temp_t.translate("Test Connection", "en", "zh")
        
        # 2. 验证成功，持久化写入 N100 配置文件
        cfg = load_server_config()
        cfg["active_channel"] = channel
        if channel in cfg["channels"]:
            for k in cfg["channels"][channel].keys():
                if k in c:
                    cfg["channels"][channel][k] = c[k]
        save_server_config(cfg)
        global _config_cache
        _config_cache = None # invalidate cache
        
        return {"status": "success", "result": test_res}
    except Exception as e:
        return {"status": "failed", "error": str(e)}

@app.post("/api/config/fetch_models")
def fetch_models(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    base_url = payload.get("base_url", "").rstrip('/')
    api_key = payload.get("api_key", "")
    
    if not base_url:
        return {"status": "failed", "error": "中转地址不能为空"}
        
    # 自动处理纯域名，补足协议头
    if not base_url.startswith("http://") and not base_url.startswith("https://"):
        base_url = "https://" + base_url

    if not _validate_url(base_url):
        return {"status": "failed", "error": "请求地址不合法 (IP 为私有、回环或保留地址)"}
        
    try:
        headers = {"Authorization": f"Bearer {api_key}"}
        res = requests.get(f"{base_url}/v1/models", headers=headers, timeout=5)
        if res.status_code == 200:
            m_list = [item["id"] for item in res.json().get("data", [])]
            return {"status": "success", "models": m_list}
        return {"status": "failed", "error": f"中转服务返回状态码 {res.status_code}"}
    except Exception as e:
        return {"status": "failed", "error": f"连接失败: {str(e)}"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app:app", host="127.0.0.1", port=18090, reload=True)

