import sys
import os
# Ensure server directory is in sys.path so imports work regardless of CWD
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from fastapi import FastAPI, Header, HTTPException
from fastapi.middleware.cors import CORSMiddleware
import requests
import time
import hashlib
from config import load_server_config, save_server_config
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator, DeepLTranslator, get_translation_runtime_metadata
from security import normalize_relay_base_url, normalize_public_base_url, request_relay_url
import logging

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
    now = time.perf_counter()
    if _config_cache is None or (now - _config_cache_time) > _CONFIG_TTL:
        _config_cache = load_server_config()
        _config_cache_time = now
    return _config_cache

def verify_token(x_api_key: str):
    cfg = get_config()
    if not x_api_key or x_api_key != cfg["client_token"]:
        logger.error(f"[verify_token] Failed! Received: '{x_api_key}', Expected: '{cfg.get('client_token')}'")
        raise HTTPException(status_code=401, detail="Unauthorized client token.")

# Translator instance cache to avoid re-creating per request (fix 4.3)
_translator_cache = {"key": None, "instance": None}
def _translator_cache_key(cfg: dict) -> str:
    channel = cfg.get("active_channel", "google")
    if channel == "new-api":
        c = cfg.get("channels", {}).get("new-api", {})
        prompt_hash = hashlib.sha256(f"{c.get('prompt', '')}\n{c.get('domain', '')}".encode("utf-8")).hexdigest()[:12]
        return f"new-api:{c.get('base_url', '')}:{c.get('api_key', '')[:8]}:{c.get('model', '')}:{prompt_hash}"
    elif channel == "baidu":
        c = cfg.get("channels", {}).get("baidu", {})
        return f"baidu:{c.get('app_id', '')}"
    elif channel == "deepl":
        c = cfg.get("channels", {}).get("deepl", {})
        return f"deepl:{c.get('endpoint', '')}:{c.get('api_key', '')[:8]}:{c.get('formality', '')}"
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
        instance = LLMTranslator(c["base_url"], c["api_key"], c["model"], c.get("prompt", ""), c.get("domain", ""), allow_private_base_url=True)
    elif channel == "baidu":
        c = cfg["channels"]["baidu"]
        logger.info("Baidu translator (AppID: %s)", c.get('app_id'))
        instance = BaiduTranslator(c["app_id"], c["secret_key"])
    elif channel == "deepl":
        c = cfg["channels"]["deepl"]
        logger.info("DeepL translator (endpoint: %s)", c.get('endpoint'))
        instance = DeepLTranslator(c.get("api_key", ""), c.get("endpoint", ""), c.get("formality", "default"))
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


def _server_config_payload(payload: dict) -> tuple[str, dict]:
    channel = payload.get("channel")
    c = payload.get("config", {}) or {}
    if channel not in {"google", "baidu", "new-api", "deepl"}:
        raise ValueError("未知翻译通道")
    if channel == "new-api":
        c = dict(c)
        c["base_url"] = normalize_relay_base_url(c.get("base_url"))
    if channel == "deepl":
        c = dict(c)
        endpoint = c.get("endpoint") or "https://api-free.deepl.com"
        c["endpoint"] = normalize_public_base_url(endpoint)
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
    channel = get_config().get("active_channel", "google")
    return {
        "status": "ok",
        "ocr": "client-local-only",
        "translation": {
            "active_channel": channel,
            **get_translation_runtime_metadata(channel),
        },
    }

@app.get("/c_hello")
async def c_hello(asker: str = ""):
    return {
        "status": "ok",
        "message": "hello",
        "asker": asker,
        "server": "local"
    }

import asyncio

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
    request_started_at = time.perf_counter()
    
    if not req.blocks:
        return {
            "status": "success",
            "translations": [],
            "cache_hits": 0,
            "channel": get_config().get("active_channel", "google"),
            "timings": {
                "total_ms": 0,
                "provider_ms": 0,
                "cache_hits": 0,
                "provider_misses": 0,
                "request_duplicates": 0,
                "preserved_hits": 0,
                "provider_failures": 0,
                "provider_fallbacks": 0,
                "provider_batch_ms": 0,
                "provider_fallback_ms": 0,
                "blocks": 0,
            },
        }
    
    texts = [block.text for block in req.blocks]
    
    # 动态获取当前激活的翻译引擎
    translator = get_active_translator()
    
    stats_ref = {
        "cache_hits": 0,
        "provider_ms": 0,
        "provider_misses": 0,
        "request_duplicates": 0,
        "preserved_hits": 0,
        "provider_failures": 0,
        "provider_fallbacks": 0,
        "provider_batch_ms": 0,
        "provider_fallback_ms": 0,
    }
    try:
        provider_started_at = time.perf_counter()
        translations = translator.translate_batch(texts, req.source_lang, req.target_lang, stats_ref)
        stats_ref["provider_ms"] += int((time.perf_counter() - provider_started_at) * 1000)
    except Exception as e:
        logger.warning("translate_text batch failed; returning aligned blank translations without slow per-item retry: %s", e)
        stats_ref["provider_failures"] += len(texts)
        stats_ref["provider_misses"] = max(0, len(texts) - stats_ref["cache_hits"])
        total_ms = int((time.perf_counter() - request_started_at) * 1000)
        cache_hits = stats_ref["cache_hits"]
        return {
            "status": "success",
            "translations": [""] * len(texts),
            "cache_hits": cache_hits,
            "channel": get_config().get("active_channel", "google"),
            "timings": {
                "total_ms": total_ms,
                "provider_ms": stats_ref["provider_ms"],
                "cache_hits": cache_hits,
                "provider_misses": stats_ref["provider_misses"],
                "request_duplicates": stats_ref["request_duplicates"],
                "preserved_hits": stats_ref["preserved_hits"],
                "provider_failures": stats_ref["provider_failures"],
                "provider_fallbacks": stats_ref["provider_fallbacks"],
                "provider_batch_ms": stats_ref["provider_batch_ms"],
                "provider_fallback_ms": stats_ref["provider_fallback_ms"],
                "blocks": len(texts),
            },
        }
                
    total_ms = int((time.perf_counter() - request_started_at) * 1000)
    cache_hits = stats_ref["cache_hits"]
    return {
        "status": "success",
        "translations": translations,
        "cache_hits": cache_hits,
        "channel": get_config().get("active_channel", "google"),
        "timings": {
            "total_ms": total_ms,
            "provider_ms": stats_ref["provider_ms"],
            "cache_hits": cache_hits,
            "provider_misses": stats_ref["provider_misses"],
            "request_duplicates": stats_ref["request_duplicates"],
            "preserved_hits": stats_ref["preserved_hits"],
            "provider_failures": stats_ref["provider_failures"],
            "provider_fallbacks": stats_ref["provider_fallbacks"],
            "provider_batch_ms": stats_ref["provider_batch_ms"],
            "provider_fallback_ms": stats_ref["provider_fallback_ms"],
            "blocks": len(texts),
        },
    }


@app.post("/api/config/test")
def test_and_save_config(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    
    try:
        channel, c = _server_config_payload(payload)
        # 1. 临时实例化对应的翻译器进行连通性验证
        if channel == "new-api":
            temp_t = LLMTranslator(c.get("base_url"), c.get("api_key"), c.get("model"), c.get("prompt", ""), c.get("domain", ""), allow_private_base_url=True)
        elif channel == "baidu":
            temp_t = BaiduTranslator(c.get("app_id"), c.get("secret_key"))
        elif channel == "deepl":
            temp_t = DeepLTranslator(c.get("api_key", ""), c.get("endpoint", ""), c.get("formality", "default"))
        else:
            temp_t = GoogleTranslator()
            
        test_res = temp_t.translate("Test Connection", "en", "zh")
        
        # 2. Persist validated settings locally
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
    return {
        "status": "success",
        "active_channel": channel,
        "config": active_cfg,
        "translation": get_translation_runtime_metadata(channel),
    }

@app.post("/api/config/fetch_models")
def fetch_models(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    base_url = payload.get("base_url", "")
    api_key = payload.get("api_key", "")

    try:
        base_url = normalize_relay_base_url(base_url)
    except ValueError as e:
        return {"status": "failed", "error": str(e)}
        
    try:
        headers = {"Authorization": f"Bearer {api_key}"}
        res = request_relay_url(requests, "GET", f"{base_url}/v1/models", headers=headers, timeout=5)
        if res.status_code == 200:
            data = res.json().get("data", [])
            m_list = [
                str(item.get("id")).strip()
                for item in data
                if isinstance(item, dict) and item.get("id")
            ]
            return {"status": "success", "models": m_list}
        return {"status": "failed", "error": f"中转服务返回状态码 {res.status_code}"}
    except Exception as e:
        return {"status": "failed", "error": f"连接失败: {str(e)}"}

if __name__ == "__main__":
    import uvicorn
    host = os.environ.get("SS_TRANSLATOR_HOST", "0.0.0.0")
    port = int(os.environ.get("SS_TRANSLATOR_PORT", "8318"))
    uvicorn.run("app:app", host=host, port=port, reload=True)
