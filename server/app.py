import sys
import os
# Ensure server directory is in sys.path so imports work regardless of CWD
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from fastapi import FastAPI, UploadFile, File, Form, Header, HTTPException, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
import requests
import cv2
import numpy as np
import time
from config import load_server_config, save_server_config
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator
from image_processor import ImageProcessor
from security import normalize_public_base_url, request_public_url
import logging
import secrets
import threading

logger = logging.getLogger(__name__)

app = FastAPI(title="Screenshot Translator API")
app.add_middleware(
    CORSMiddleware,
    allow_origins=[
        "http://localhost:1420", 
        "http://127.0.0.1:1420", 
        "tauri://localhost",
        "https://tauri.localhost",
        "http://tauri.localhost"
    ],
    allow_credentials=False,
    allow_methods=["GET", "POST", "OPTIONS"],
    allow_headers=["*"],
)

# 默认不加载 heavy OCR 模型以加速服务启动，首个翻译请求来时会触发懒加载
processor = ImageProcessor(load_ocr=False)

# 🌟 后台异步加载并预热 OCR 模型以消除首次请求的冷启动卡顿
def warm_up_ocr_async():
    try:
        time.sleep(1.5)
        logger.info("[OCR Warmup] Loading PaddleOCR models...")
        processor._ensure_ocr()
        logger.info("[OCR Warmup] PaddleOCR models warmed up and ready.")
    except Exception as e:
        logger.error("[OCR Warmup] Background warmup failed: %s", e, exc_info=True)

threading.Thread(target=warm_up_ocr_async, daemon=True).start()

# 🔑 启动时打印当前 client_token，供运维人员在客户端配置
_startup_cfg = load_server_config()
_token_val = _startup_cfg['client_token']
logger.warning(f"[Security] 当前 client_token: {_token_val}")
logger.warning("[Security] 请将此 token 填入客户端「系统设置 → 令牌」中，或通过环境变量 SS_TRANSLATOR_TOKEN 覆盖。")
del _startup_cfg, _token_val


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

# Translator instance cache to avoid re-creating per request (fix 4.3)
_translator_cache = {"key": None, "instance": None}
_translate_meta_cache = {}
_translate_meta_lock = threading.Lock()
_TRANSLATE_META_TTL = 600.0

def _translator_cache_key(cfg: dict) -> str:
    channel = cfg.get("active_channel", "google")
    if channel == "new-api":
        c = cfg.get("channels", {}).get("new-api", {})
        return f"new-api:{c.get('base_url', '')}:{c.get('api_key', '')[:8]}:{c.get('model', '')}"
    elif channel == "baidu":
        c = cfg.get("channels", {}).get("baidu", {})
        return f"baidu:{c.get('app_id', '')}"
    return "google"

def get_active_translator():
    cfg = get_config()
    channel = cfg.get("active_channel", "google")
    cache_key = _translator_cache_key(cfg)
    if _translator_cache["key"] == cache_key and _translator_cache["instance"] is not None:
        logger.debug("Reusing cached translator: %s", channel)
        return _translator_cache["instance"]
    logger.debug("Creating translator for channel: %s", channel)
    if channel == "new-api":
        c = cfg["channels"]["new-api"]
        logger.info("LLM translator (relay: %s, model: %s)", c.get('base_url'), c.get('model'))
        instance = LLMTranslator(c["base_url"], c["api_key"], c["model"])
    elif channel == "baidu":
        c = cfg["channels"]["baidu"]
        logger.info("Baidu translator (AppID: %s)", c.get('app_id'))
        instance = BaiduTranslator(c["app_id"], c["secret_key"])
    else:
        logger.info("Google free translator (no credentials)")
        instance = GoogleTranslator()
    _translator_cache["key"] = cache_key
    _translator_cache["instance"] = instance
    return instance

def invalidate_config_cache():
    global _config_cache, _config_cache_time
    _config_cache = None
    _config_cache_time = 0.0
    _translator_cache["key"] = None
    _translator_cache["instance"] = None


def _store_translate_meta(stats: dict) -> str | None:
    texts_json = stats.get("texts_json")
    if not texts_json:
        return None
    meta_id = secrets.token_urlsafe(16)
    with _translate_meta_lock:
        now = time.time()
        expired = [key for key, item in _translate_meta_cache.items() if item["expires_at"] <= now]
        for key in expired:
            _translate_meta_cache.pop(key, None)
        _translate_meta_cache[meta_id] = {
            "texts": texts_json,
            "expires_at": now + _TRANSLATE_META_TTL,
        }
    return meta_id


def _server_config_payload(payload: dict) -> tuple[str, dict]:
    channel = payload.get("channel")
    c = payload.get("config", {}) or {}
    if channel not in {"google", "baidu", "new-api"}:
        raise ValueError("未知翻译通道")
    if channel == "new-api":
        c = dict(c)
        c["base_url"] = normalize_public_base_url(c.get("base_url"))
    return channel, c


def _save_channel_config(channel: str, channel_config: dict):
    cfg = load_server_config()
    cfg["active_channel"] = channel
    if channel in cfg.get("channels", {}):
        for key in cfg["channels"][channel].keys():
            if key in channel_config:
                cfg["channels"][channel][key] = channel_config[key]
    save_server_config(cfg)
    invalidate_config_cache()

@app.get("/api/health")
async def health_check():
    return {"status": "ok", "ocr_ready": processor.ocr_ready}

@app.get("/c_hello")
async def c_hello(asker: str = ""):
    return {
        "status": "ok",
        "message": "hello",
        "asker": asker,
        "server": "local"
    }

import asyncio

@app.post("/api/translate")
async def translate_image(
    image: UploadFile = File(...),
    target_lang: str = Form("zh"),
    x_api_key: str = Header(None)
):
    verify_token(x_api_key)
    img_bytes = await image.read()
    
    # 动态获取当前激活的翻译引擎
    translator = get_active_translator()
    def translator_batch_fn(texts, stats_ref):
        return translator.translate_batch(texts, "auto", target_lang or "zh", stats_ref)
        
    try:
        out_bytes, stats = await asyncio.to_thread(
            processor.process_and_draw, img_bytes, translator_batch_fn, config=get_config(), target_lang=(target_lang or "zh")
        )
        
        # 终端漂亮的可视化耗时报告
        if get_config().get("debug_trace", False):
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
                logger.info(line)

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
            "X-Trace-Channel": str(get_config().get("active_channel", "google")),
            "X-Ocr-Ready": "true" if stats.get("ocr_ready", False) else "false",
            "X-Ocr-Cache-Hit": "true" if stats.get("ocr_cache_hit", False) else "false"
        }
        meta_id = _store_translate_meta(stats)
        if meta_id:
            headers["X-Translate-Meta-Id"] = meta_id
            if len(stats.get("texts_json", "")) <= 6000:
                headers["X-Translate-Texts"] = stats["texts_json"]
            
        return Response(content=out_bytes, media_type="image/png", headers=headers)
    except Exception as e:
        logger.exception("translate_image failed")
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

        ocr_result = processor.run_ocr(img_cv, cls=True)

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
        logger.warning("translate_text batch failed, falling back to single: %s", e)
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


@app.get("/api/translate/meta/{meta_id}")
def translate_meta(meta_id: str, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    with _translate_meta_lock:
        item = _translate_meta_cache.get(meta_id)
        if not item or item["expires_at"] <= time.time():
            _translate_meta_cache.pop(meta_id, None)
            raise HTTPException(status_code=404, detail="Translate metadata expired or not found.")
        return {"status": "success", "texts": item["texts"]}

@app.post("/api/config/test")
def test_and_save_config(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    
    try:
        channel, c = _server_config_payload(payload)
        # 1. 临时实例化对应的翻译器进行连通性验证
        if channel == "new-api":
            temp_t = LLMTranslator(c.get("base_url"), c.get("api_key"), c.get("model"))
        elif channel == "baidu":
            temp_t = BaiduTranslator(c.get("app_id"), c.get("secret_key"))
        else:
            temp_t = GoogleTranslator()
            
        test_res = temp_t.translate("Test Connection", "en", "zh")
        
        # 2. 验证成功，持久化写入 N100 配置文件
        _save_channel_config(channel, c)
        
        return {"status": "success", "result": test_res}
    except Exception as e:
        return {"status": "failed", "error": str(e)}

@app.post("/api/config/save")
def save_channel_config(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    try:
        channel, c = _server_config_payload(payload)
        _save_channel_config(channel, c)
        return {"status": "success", "active_channel": channel}
    except Exception as e:
        return {"status": "failed", "error": str(e)}

@app.get("/api/config/current")
def current_config(x_api_key: str = Header(None)):
    verify_token(x_api_key)
    cfg = get_config()
    channel = cfg.get("active_channel", "google")
    active_cfg = dict(cfg.get("channels", {}).get(channel, {}))
    for secret_key in ("api_key", "secret_key"):
        if active_cfg.get(secret_key):
            active_cfg[secret_key] = "***"
    return {"status": "success", "active_channel": channel, "config": active_cfg}

@app.post("/api/config/fetch_models")
def fetch_models(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    base_url = payload.get("base_url", "")
    api_key = payload.get("api_key", "")

    try:
        base_url = normalize_public_base_url(base_url)
    except ValueError as e:
        return {"status": "failed", "error": str(e)}
        
    try:
        headers = {"Authorization": f"Bearer {api_key}"}
        res = request_public_url(requests, "GET", f"{base_url}/v1/models", headers=headers, timeout=5)
        if res.status_code == 200:
            m_list = [item["id"] for item in res.json().get("data", [])]
            return {"status": "success", "models": m_list}
        return {"status": "failed", "error": f"中转服务返回状态码 {res.status_code}"}
    except Exception as e:
        return {"status": "failed", "error": f"连接失败: {str(e)}"}

if __name__ == "__main__":
    import uvicorn
    host = os.environ.get("SS_TRANSLATOR_HOST", "0.0.0.0")
    port = int(os.environ.get("SS_TRANSLATOR_PORT", "8318"))
    uvicorn.run("app:app", host=host, port=port, reload=True)

