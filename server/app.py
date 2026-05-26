from fastapi import FastAPI, UploadFile, File, Header, HTTPException, Response
from fastapi.middleware.cors import CORSMiddleware
import requests
import json
import cv2
import numpy as np
from paddleocr import PaddleOCR
from config import load_server_config, save_server_config
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator
from image_processor import ImageProcessor

app = FastAPI(title="Screenshot Translator API")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# 默认不加载 heavy OCR 模型以加速服务启动，首个翻译请求来时会触发懒加载
processor = ImageProcessor(load_ocr=False)

def verify_token(x_api_key: str):
    cfg = load_server_config()
    if not x_api_key or x_api_key != cfg["client_token"]:
        raise HTTPException(status_code=401, detail="Unauthorized client token.")

def get_active_translator():
    cfg = load_server_config()
    channel = cfg["active_channel"]
    if channel == "new-api":
        c = cfg["channels"]["new-api"]
        return LLMTranslator(c["base_url"], c["api_key"], c["model"])
    elif channel == "baidu":
        c = cfg["channels"]["baidu"]
        return BaiduTranslator(c["app_id"], c["secret_key"])
    return GoogleTranslator()

@app.post("/api/translate")
async def translate_image(image: UploadFile = File(...), x_api_key: str = Header(None)):
    verify_token(x_api_key)
    img_bytes = await image.read()
    
    # 动态获取当前激活的翻译引擎
    translator = get_active_translator()
    def translator_batch_fn(texts):
        return translator.translate_batch(texts, "auto", "zh")
        
    out_bytes = processor.process_and_draw(img_bytes, translator_batch_fn)
    return Response(content=out_bytes, media_type="image/png")

@app.post("/api/ocr")
async def ocr_image(image: UploadFile = File(...), x_api_key: str = Header(None)):
    verify_token(x_api_key)
    img_bytes = await image.read()
    
    nparr = np.frombuffer(img_bytes, np.uint8)
    img_cv = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    
    if processor.ocr is None:
        processor.ocr = PaddleOCR(lang="ch")
        
    ocr_result = processor.ocr.ocr(img_cv, cls=True)
    
    results = []
    if ocr_result and ocr_result[0]:
        for line in ocr_result[0]:
            box = line[0] # [[x1, y1], [x2, y1], [x2, y2], [x1, y2]]
            text = line[1][0]
            confidence = float(line[1][1])
            results.append({
                "box": box,
                "text": text,
                "confidence": confidence
            })
            
    return {"status": "success", "ocr": results}

@app.post("/api/config/test")
async def test_and_save_config(payload: dict, x_api_key: str = Header(None)):
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
        
        return {"status": "success", "result": test_res}
    except Exception as e:
        return {"status": "failed", "error": str(e)}

@app.post("/api/config/fetch_models")
async def fetch_models(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    base_url = payload.get("base_url", "").rstrip('/')
    api_key = payload.get("api_key", "")
    
    if not base_url:
        return {"status": "failed", "error": "中转地址不能为空"}
        
    # 自动处理纯域名，补足协议头
    if not base_url.startswith("http://") and not base_url.startswith("https://"):
        base_url = "https://" + base_url
        
    try:
        headers = {"Authorization": f"Bearer {api_key}"}
        res = requests.get(f"{base_url}/v1/models", headers=headers, timeout=5)
        if res.status_code == 200:
            m_list = [item["id"] for item in res.json().get("data", [])]
            return {"status": "success", "models": m_list}
        return {"status": "failed", "error": f"中转服务返回状态码 {res.status_code}"}
    except Exception as e:
        return {"status": "failed", "error": f"连接失败: {str(e)}"}
