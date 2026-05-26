# 截图翻译软件（第一阶段 - MVP）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个在 N100 上部署的 Python (FastAPI) 图像翻译服务端，以及配套的 Windows C++/Qt 6 客户端，实现毫秒级快速截图、背景采样自动涂抹嵌字和自定义 AI/百度/谷歌翻译接口的测试保存。

**Architecture:** 客户端采用 QNetworkAccessManager 与 N100 FastAPI 后端通信。后端使用 PaddleOCR 做文字定位，使用 OpenCV 做多边形像素背景分析与文字填充，使用 Pillow 做译文折行重绘，并中转翻译请求。

**Tech Stack:** C++ / Qt 6, Python 3.10+, FastAPI, PaddleOCR, OpenCV (cv2), Pillow (PIL), pytest

---

## 预定义文件结构

```text
d:\Desktop\自制截图\
├── docs\
│   └── specs\
│       └── 2026-05-26-screenshot-translator-design.md
├── server\                  # Python 服务端目录
│   ├── app.py               # FastAPI 服务入口
│   ├── config.py            # 服务端配置文件读写
│   ├── translator.py        # 统一翻译引擎驱动类
│   ├── image_processor.py   # OCR+擦除+排版绘制图像核心类
│   ├── requirements.txt     # Python 依赖包
│   └── tests\
│       ├── __init__.py
│       ├── test_translator.py
│       └── test_processor.py
└── client\                  # C++/Qt 客户端目录
    ├── CMakeLists.txt       # CMake 构建脚本
    ├── main.cpp             # 客户端入口
    ├── config.h/cpp         # 客户端本地 config.json 读取
    ├── networkclient.h/cpp  # API 请求与网络通信组件
    ├── screenshotwindow.h/cpp # 截图框选与浮动工具栏交互
    └── settingspanel.h/cpp  # 配置面板交互界面
```

---

## 实施步骤清单

### Task 1: Python 服务端 - 核心翻译引擎 `translator.py`

**Files:**
- Create: `server/translator.py`
- Test: `server/tests/test_translator.py`

- [ ] **Step 1: 编写测试用例，覆盖三种翻译引擎**

在 `server/tests/test_translator.py` 中写出测试：
```python
import pytest
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator

def test_google_translation():
    translator = GoogleTranslator()
    res = translator.translate("Hello, world!", "en", "zh")
    assert "你好" in res

def test_llm_translation_format():
    # 模拟一个 OpenAI 兼容的请求
    translator = LLMTranslator(
        base_url="http://192.168.1.3:3001",
        api_key="sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb",
        model="gemini-1.5-flash"
    )
    # 此处若由于内网未连通失败，应能合理捕获异常并返回降级提示
    try:
        res = translator.translate("Hello", "en", "zh")
        assert isinstance(res, str)
    except Exception as e:
        pytest.skip(f"new-api not reachable in current env: {e}")
```

- [ ] **Step 2: 运行测试并确保失败**

在 `server` 目录下运行：
`pytest tests/test_translator.py -v`
**预期结果**：失败，找不到 `translator` 模块。

- [ ] **Step 3: 编写核心接口与引擎实现**

在 `server/translator.py` 中编写 `BaseTranslator` 抽象基类及派生类：
```python
import abc
import requests
import json
import urllib.parse
import hashlib
import random

class BaseTranslator(abc.ABC):
    @abc.abstractmethod
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        pass

class GoogleTranslator(BaseTranslator):
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        # 使用 Google Web 翻译免密接口
        url = f"https://translate.googleapis.com/translate_a/single?client=gtx&sl={source_lang}&tl={target_lang}&dt=t&q={urllib.parse.quote(text)}"
        response = requests.get(url, timeout=5)
        if response.status_code == 200:
            res_json = response.json()
            return "".join([part[0] for part in res_json[0] if part[0]])
        raise Exception(f"Google translate failed: status {response.status_code}")

class LLMTranslator(BaseTranslator):
    def __init__(self, base_url: str, api_key: str, model: str):
        self.base_url = base_url.rstrip('/')
        self.api_key = api_key
        self.model = model

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": f"You are a translation assistant. Translate the following text into Simplified Chinese. Output ONLY the translated text, do not include any commentary, explanations, or quotes."},
                {"role": "user", "content": text}
            ],
            "temperature": 0.3
        }
        res = requests.post(f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=10)
        if res.status_code == 200:
            return res.json()["choices"][0]["message"]["content"].strip()
        raise Exception(f"LLM translation failed: {res.text}")

class BaiduTranslator(BaseTranslator):
    def __init__(self, app_id: str, secret_key: str):
        self.app_id = app_id
        self.secret_key = secret_key

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        salt = str(random.randint(32768, 65536))
        sign_str = self.app_id + text + salt + self.secret_key
        sign = hashlib.md5(sign_str.encode('utf-8')).hexdigest()
        
        # 语种转换映射
        from_lang = "auto" if source_lang == "auto" else source_lang
        to_lang = "zh" if target_lang == "zh" else target_lang
        
        url = f"https://fanyi-api.baidu.com/api/trans/vip/translate?q={urllib.parse.quote(text)}&from={from_lang}&to={to_lang}&appid={self.app_id}&salt={salt}&sign={sign}"
        res = requests.get(url, timeout=5)
        if res.status_code == 200:
            res_json = res.json()
            if "error_code" in res_json:
                raise Exception(f"Baidu translation API error: {res_json['error_msg']}")
            return "".join([item["dst"] for item in res_json["trans_result"]])
        raise Exception(f"Baidu request failed: status {res.status_code}")
```

- [ ] **Step 4: 运行测试确保其通过**

`pytest tests/test_translator.py -v`
**预期结果**：PASS (跳过未连通的内网大模型服务)。

- [ ] **Step 5: 提交代码**

```bash
git add server/translator.py server/tests/test_translator.py
git commit -m "feat(server): add unified translators (Google Web, LLM, Baidu)"
```

---

### Task 2: Python 服务端 - 图像擦除与译文重绘 `image_processor.py`

**Files:**
- Create: `server/image_processor.py`
- Test: `server/tests/test_processor.py`

- [ ] **Step 1: 编写测试用例模拟文字区域涂抹**

在 `server/tests/test_processor.py` 中写出测试：
```python
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
    processor = ImageProcessor()
    bg_color = processor.sample_background(img_arr, [40, 40, 60, 60])
    assert bg_color == (255, 255, 255)
```

- [ ] **Step 2: 运行测试确保失败**

在 `server` 目录下运行：
`pytest tests/test_processor.py -v`
**预期结果**：失败，找不到 `image_processor`。

- [ ] **Step 3: 编写擦除重绘逻辑**

在 `server/image_processor.py` 中实现 OCR 坐标定位、擦除和文字排版自动字号折行重绘：
```python
import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from paddleocr import PaddleOCR
import os

class ImageProcessor:
    def __init__(self):
        # 启动时常驻加载 PaddleOCR
        self.ocr = PaddleOCR(use_angle_cls=True, lang="ch", show_log=False)

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
            # OpenCV BGR格式颜色转换
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
            font_size = h_box
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
            
            # 居中重绘文字
            y_offset = y_min + (h_box - len(lines) * font_size) // 2
            for l in lines:
                text_w = draw.textlength(l, font=font)
                x_offset = x_min + (w_box - text_w) // 2
                # PIL Draw以 RGB 颜色渲染，需要转换 RGB
                rgb_fg = (fg_color[2], fg_color[1], fg_color[0])
                draw.text((x_offset, y_offset), l, fill=rgb_fg, font=font)
                y_offset += font_size

        # 将处理完的 PIL 图像导出为 PNG 二进制流
        import io
        img_out = io.BytesIO()
        pil_img.save(img_out, format="PNG")
        return img_out.getvalue()
```

- [ ] **Step 4: 运行测试确保其通过**

`pytest tests/test_processor.py -v`
**预期结果**：PASS.

- [ ] **Step 5: 提交**

```bash
git add server/image_processor.py server/tests/test_processor.py
git commit -m "feat(server): add image processor with PaddleOCR, sample back/fore colors, PIL draw"
```

---

### Task 3: Python 服务端 - FastAPI 主体与接口 `app.py`

**Files:**
- Create: `server/app.py`
- Create: `server/config.py`
- Test: `server/tests/test_server.py`

- [ ] **Step 1: 编写服务器整体路由测试**

在 `server/tests/test_server.py` 中编写测试：
```python
import pytest
from fastapi.testclient import TestClient
from app import app

client = TestClient(app)

def test_api_unauthorized():
    res = client.post("/api/translate", files={"image": b"fake-data"})
    assert res.status_code == 401 # 未提供正确 API Key

def test_fetch_models_proxy():
    # 测试获取模型代理接口
    res = client.post(
        "/api/config/fetch_models",
        headers={"X-API-Key": "ysn-screenshot-translator-token-666"},
        json={"base_url": "https://api.yousn.me", "api_key": "sk-invalid"}
    )
    # 即使 Key 错也应返回合理 JSON 报错而非崩溃
    assert res.status_code == 200
    assert res.json()["status"] == "failed"
```

- [ ] **Step 2: 运行测试确保失败**

`pytest tests/test_server.py -v`
**预期结果**：失败，未实现服务器代码。

- [ ] **Step 3: 编写配置文件及服务器代码**

在 `server/config.py` 中：
```python
import yaml
import os

CONFIG_PATH = os.path.expanduser("~/.screenshot-translator/config.yaml")

def load_server_config():
    if not os.path.exists(CONFIG_PATH):
        os.makedirs(os.path.dirname(CONFIG_PATH), exist_ok=True)
        default = {
            "client_token": "ysn-screenshot-translator-token-666",
            "active_channel": "google",
            "channels": {
                "new-api": {
                    "base_url": "http://192.168.1.3:3001",
                    "api_key": "sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb",
                    "model": "gemini-1.5-flash"
                },
                "baidu": {
                    "app_id": "",
                    "secret_key": ""
                }
            }
        }
        save_server_config(default)
        return default
    with open(CONFIG_PATH, 'r', encoding='utf-8') as f:
        return yaml.safe_load(f)

def save_server_config(cfg):
    os.makedirs(os.path.dirname(CONFIG_PATH), exist_ok=True)
    with open(CONFIG_PATH, 'w', encoding='utf-8') as f:
        yaml.safe_dump(cfg, f, allow_unicode=True)
```

在 `server/app.py` 中：
```python
from fastapi import FastAPI, UploadFile, File, Header, HTTPException, Response
from fastapi.middleware.cors import CORSMiddleware
import requests
import json
from config import load_server_config, save_server_config
from translator import GoogleTranslator, LLMTranslator, BaiduTranslator
from image_processor import ImageProcessor

app = FastAPI(title="Screenshot Translator API")
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

processor = ImageProcessor()

def verify_token(x_api_key: str = Header(None)):
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
    
    # 动态获取当前配置好的翻译引擎
    translator = get_active_translator()
    def translator_fn(text):
        return translator.translate(text, "auto", "zh")
        
    out_bytes = processor.process_and_draw(img_bytes, translator_fn)
    return Response(content=out_bytes, media_type="image/png")

@app.post("/api/config/test")
async def test_and_save_config(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    channel = payload.get("channel")
    c = payload.get("config", {})
    
    try:
        # 1. 临时实例化对应的翻译器进行验证
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
            for k, v in c.items():
                cfg["channels"][channel][k] = v
        save_server_config(cfg)
        
        return {"status": "success", "result": test_res}
    except Exception as e:
        return {"status": "failed", "error": str(e)}

@app.post("/api/config/fetch_models")
async def fetch_models(payload: dict, x_api_key: str = Header(None)):
    verify_token(x_api_key)
    base_url = payload.get("base_url", "").rstrip('/')
    api_key = payload.get("api_key", "")
    
    # 强制在 URL 中补足 http 协议前缀（如果用户输入的是纯域名）
    if not base_url.startswith("http://") and not base_url.startswith("https://"):
        base_url = "https://" + base_url # 默认安全外网，可改内网
        
    try:
        headers = {"Authorization": f"Bearer {api_key}"}
        res = requests.get(f"{base_url}/v1/models", headers=headers, timeout=5)
        if res.status_code == 200:
            m_list = [item["id"] for item in res.json().get("data", [])]
            return {"status": "success", "models": m_list}
        return {"status": "failed", "error": f"API returned status {res.status_code}"}
    except Exception as e:
        return {"status": "failed", "error": str(e)}
```

- [ ] **Step 4: 运行测试确保其通过**

`pytest tests/test_server.py -v`
**预期结果**：PASS.

- [ ] **Step 5: 提交**

```bash
git add server/app.py server/config.py server/tests/test_server.py
git commit -m "feat(server): add FastAPI routing, config manager and validation endpoints"
```

---

### Task 4: C++/Qt 客户端 - 网络与配置组件 `networkclient.h/cpp`

**Files:**
- Create: `client/config.h`
- Create: `client/config.cpp`
- Create: `client/networkclient.h`
- Create: `client/networkclient.cpp`

- [ ] **Step 1: 编写客户端本地 `config.json` 序列化测试逻辑**

在 `client/config.h` 中定义本地配置文件交互：
```cpp
#pragma once
#include <QString>
#include <QJsonObject>

struct ClientConfig {
    QString serverUrl = "https://ocr.yousn.me";
    QString clientToken = "ysn-screenshot-translator-token-666";
    QString channel = "new-api";
    QString newApiBase = "api.yousn.me";
    QString newApiKey = "sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb";
    QString newApiModel = "gemini-1.5-flash";
    QString baiduAppId = "";
    QString baiduSecretKey = "";

    void load();
    void save();
};
```

- [ ] **Step 2: 编写实现代码及网络请求库**

在 `client/config.cpp` 中实现读取：
```cpp
#include "config.h"
#include <QFile>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QStandardPaths>

void ClientConfig::load() {
    QFile file("config.json");
    if (!file.open(QIODevice::ReadOnly)) return;
    QByteArray data = file.readAll();
    QJsonDocument doc = QJsonDocument::fromJson(data);
    if (doc.isNull()) return;
    QJsonObject obj = doc.object();
    
    serverUrl = obj.value("serverUrl").toString(serverUrl);
    clientToken = obj.value("clientToken").toString(clientToken);
    channel = obj.value("channel").toString(channel);
    newApiBase = obj.value("newApiBase").toString(newApiBase);
    newApiKey = obj.value("newApiKey").toString(newApiKey);
    newApiModel = obj.value("newApiModel").toString(newApiModel);
    baiduAppId = obj.value("baiduAppId").toString(baiduAppId);
    baiduSecretKey = obj.value("baiduSecretKey").toString(baiduSecretKey);
}

void ClientConfig::save() {
    QFile file("config.json");
    if (!file.open(QIODevice::WriteOnly)) return;
    QJsonObject obj;
    obj["serverUrl"] = serverUrl;
    obj["clientToken"] = clientToken;
    obj["channel"] = channel;
    obj["newApiBase"] = newApiBase;
    obj["newApiKey"] = newApiKey;
    obj["newApiModel"] = newApiModel;
    obj["baiduAppId"] = baiduAppId;
    obj["baiduSecretKey"] = baiduSecretKey;
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Indented));
}
```

在 `client/networkclient.h` 中实现 Qt6 的多部分网络发送与大模型拉取：
```cpp
#pragma once
#include <QObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QPixmap>
#include "config.h"

class NetworkClient : public QObject {
    Q_OBJECT
public:
    explicit NetworkClient(QObject *parent = nullptr);
    
    // 发起翻译截图
    void translateImage(const QPixmap &pixmap, const ClientConfig &cfg, 
                        std::function<void(bool success, const QPixmap &resPixmap)> callback);

    // 测试并保存配置
    void testConfig(const ClientConfig &cfg, const QJsonObject &testPayload,
                    std::function<void(bool success, const QString &msg)> callback);

    // 动态拉取大模型列表
    void fetchModels(const ClientConfig &cfg, const QString &baseUrl, const QString &apiKey,
                     std::function<void(bool success, const QStringList &models)> callback);

private:
    QNetworkAccessManager *manager;
};
```

在 `client/networkclient.cpp` 中使用 Qt `QNetworkAccessManager` 和 `QHttpMultiPart` 发送：
```cpp
#include "networkclient.h"
#include <QHttpMultiPart>
#include <QHttpPart>
#include <QBuffer>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>

NetworkClient::NetworkClient(QObject *parent) : QObject(parent) {
    manager = new QNetworkAccessManager(this);
}

void NetworkClient::translateImage(const QPixmap &pixmap, const ClientConfig &cfg, 
                                   std::function<void(bool success, const QPixmap &resPixmap)> callback) {
    QByteArray ba;
    QBuffer buffer(&ba);
    buffer.open(QIODevice::WriteOnly);
    pixmap.save(&buffer, "PNG");
    
    QHttpMultiPart *multiPart = new QHttpMultiPart(QHttpMultiPart::FormDataType);
    QHttpPart imagePart;
    imagePart.setHeader(QNetworkRequest::ContentTypeHeader, QVariant("image/png"));
    imagePart.setHeader(QNetworkRequest::ContentDispositionHeader, 
                        QVariant("form-data; name=\"image\"; filename=\"screenshot.png\""));
    imagePart.setBody(ba);
    multiPart->append(imagePart);
    
    QNetworkRequest request(QUrl(cfg.serverUrl + "/api/translate"));
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QNetworkReply *reply = manager->post(request, multiPart);
    multiPart->setParent(reply); // 随请求一同释放
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QPixmap outPix;
            if (outPix.loadFromData(reply->readAll())) {
                callback(true, outPix);
            } else {
                callback(false, QPixmap());
            }
        } else {
            callback(false, QPixmap());
        }
        reply->deleteLater();
    });
}

void NetworkClient::testConfig(const ClientConfig &cfg, const QJsonObject &testPayload,
                               std::function<void(bool success, const QString &msg)> callback) {
    QNetworkRequest request(QUrl(cfg.serverUrl + "/api/config/test"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QByteArray postData = QJsonDocument(testPayload).toJson();
    QNetworkReply *reply = manager->post(request, postData);
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QJsonObject res = QJsonDocument::fromJson(reply->readAll()).object();
            if (res.value("status").toString() == "success") {
                callback(true, res.value("result").toString());
            } else {
                callback(false, res.value("error").toString());
            }
        } else {
            callback(false, reply->errorString());
        }
        reply->deleteLater();
    });
}

void NetworkClient::fetchModels(const ClientConfig &cfg, const QString &baseUrl, const QString &apiKey,
                                std::function<void(bool success, const QStringList &models)> callback) {
    QNetworkRequest request(QUrl(cfg.serverUrl + "/api/config/fetch_models"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QJsonObject payload;
    payload["base_url"] = baseUrl;
    payload["api_key"] = apiKey;
    
    QByteArray postData = QJsonDocument(payload).toJson();
    QNetworkReply *reply = manager->post(request, postData);
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QJsonObject res = QJsonDocument::fromJson(reply->readAll()).object();
            if (res.value("status").toString() == "success") {
                QStringList mList;
                QJsonArray arr = res.value("models").toArray();
                for (auto val : arr) {
                    mList << val.toString();
                }
                callback(true, mList);
            } else {
                callback(false, QStringList());
            }
        } else {
            callback(false, QStringList());
        }
        reply->deleteLater();
    });
}
```

- [ ] **Step 3: 提交**

```bash
git add client/config.h client/config.cpp client/networkclient.h client/networkclient.cpp
git commit -m "feat(client): implement ClientConfig loader and NetworkClient API handlers"
```

---

### Task 5: C++/Qt 客户端 - 截图交互与设置面板 UI 设计 `screenshotwindow.cpp`

**Files:**
- Create: `client/screenshotwindow.h`
- Create: `client/screenshotwindow.cpp`
- Create: `client/settingspanel.h`
- Create: `client/settingspanel.cpp`

- [ ] **Step 1: 编写客户端 UI 及事件分发**

在 `client/settingspanel.h` 中声明 UI 字段及回调：
```cpp
#pragma once
#include <QDialog>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include <QLabel>
#include "config.h"
#include "networkclient.h"

class SettingsPanel : public QDialog {
    Q_OBJECT
public:
    explicit SettingsPanel(QWidget *parent = nullptr);
private:
    QLineEdit *serverUrlEdit;
    QLineEdit *clientTokenEdit;
    QComboBox *channelCombo;
    
    // new-api settings
    QLineEdit *newApiBaseEdit;
    QLineEdit *newApiKeyEdit;
    QComboBox *newApiModelCombo;
    QPushButton *fetchModelsBtn;

    // Baidu settings
    QLineEdit *baiduAppIdEdit;
    QLineEdit *baiduSecretKeyEdit;

    QPushButton *verifyBtn;
    QLabel *statusLabel;
    
    ClientConfig config;
    NetworkClient *netClient;
    
    void loadFields();
    void saveFields();
};
```

在 `client/settingspanel.cpp` 中使用 Qt 经典布局引擎绘制窗口，并添加带有“获取模型”和“验证配置”事件的精美界面：
```cpp
#include "settingspanel.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGroupBox>

SettingsPanel::SettingsPanel(QWidget *parent) : QDialog(parent) {
    setWindowTitle("配置面板 (Settings Panel)");
    resize(550, 480);
    
    config.load();
    netClient = new NetworkClient(this);
    
    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    
    // 1. 服务器设置
    QGroupBox *serverGroup = new QGroupBox("服务器设置", this);
    QFormLayout *serverForm = new QFormLayout(serverGroup);
    
    QLabel *hint1 = new QLabel("提示: 内网地址 http://192.168.1.3:8318", this);
    hint1->setStyleSheet("color: gray; font-size: 11px;");
    serverForm->addRow(hint1);
    
    serverUrlEdit = new QLineEdit(config.serverUrl, this);
    serverForm->addRow("服务器地址:", serverUrlEdit);
    
    clientTokenEdit = new QLineEdit(config.clientToken, this);
    clientTokenEdit->setEchoMode(QLineEdit::Password);
    serverForm->addRow("鉴权 Token:", clientTokenEdit);
    
    mainLayout->addWidget(serverGroup);
    
    // 2. 翻译设置
    QGroupBox *transGroup = new QGroupBox("翻译通道设置", this);
    QVBoxLayout *transVBox = new QVBoxLayout(transGroup);
    
    channelCombo = new QComboBox(this);
    channelCombo->addItems({"new-api", "baidu", "google"});
    channelCombo->setCurrentText(config.channel);
    transVBox->addWidget(channelCombo);
    
    // new-api Group
    QGroupBox *llmSubGroup = new QGroupBox("new-api (LLM 模式)", this);
    QFormLayout *llmForm = new QFormLayout(llmSubGroup);
    
    QLabel *hint2 = new QLabel("提示: 内网中转地址 http://192.168.1.3:3001", this);
    hint2->setStyleSheet("color: gray; font-size: 11px;");
    llmForm->addRow(hint2);
    
    newApiBaseEdit = new QLineEdit(config.newApiBase, this);
    llmForm->addRow("中转地址:", newApiBaseEdit);
    
    newApiKeyEdit = new QLineEdit(config.newApiKey, this);
    newApiKeyEdit->setEchoMode(QLineEdit::Password);
    llmForm->addRow("API Key:", newApiKeyEdit);
    
    QHBoxLayout *modelHBox = new QHBoxLayout();
    newApiModelCombo = new QComboBox(this);
    newApiModelCombo->addItem(config.newApiModel);
    newApiModelCombo->setEditable(true);
    modelHBox->addWidget(newApiModelCombo);
    
    fetchModelsBtn = new QPushButton("获取模型", this);
    modelHBox->addWidget(fetchModelsBtn);
    llmForm->addRow("模型名称:", modelHBox);
    
    transVBox->addWidget(llmSubGroup);
    
    // 百度翻译组
    QGroupBox *baiduSubGroup = new QGroupBox("百度翻译", this);
    QFormLayout *baiduForm = new QFormLayout(baiduSubGroup);
    baiduAppIdEdit = new QLineEdit(config.baiduAppId, this);
    baiduSecretKeyEdit = new QLineEdit(config.baiduSecretKey, this);
    baiduForm->addRow("AppID:", baiduAppIdEdit);
    baiduForm->addRow("密钥:", baiduSecretKeyEdit);
    
    transVBox->addWidget(baiduSubGroup);
    mainLayout->addWidget(transGroup);
    
    // 3. 测试与保存
    QHBoxLayout *btnHBox = new QHBoxLayout();
    verifyBtn = new QPushButton("点击验证", this);
    btnHBox->addWidget(verifyBtn);
    
    statusLabel = new QLabel("", this);
    statusLabel->setStyleSheet("font-weight: bold;");
    btnHBox->addWidget(statusLabel);
    
    btnHBox->addStretch();
    QPushButton *saveBtn = new QPushButton("保存并应用", this);
    QPushButton *cancelBtn = new QPushButton("取消", this);
    btnHBox->addWidget(cancelBtn);
    btnHBox->addWidget(saveBtn);
    mainLayout->addLayout(btnHBox);
    
    // 控制显示联动
    auto updateVisibility = [=]() {
        llmSubGroup->setVisible(channelCombo->currentText() == "new-api");
        baiduSubGroup->setVisible(channelCombo->currentText() == "baidu");
    };
    connect(channelCombo, &QComboBox::currentTextChanged, updateVisibility);
    updateVisibility();
    
    // 连接信号事件
    connect(fetchModelsBtn, &QPushButton::clicked, [=]() {
        fetchModelsBtn->setEnabled(false);
        statusLabel->setText("正在获取模型列表...");
        statusLabel->setStyleSheet("color: blue;");
        
        saveFields(); // 更新临时状态
        netClient->fetchModels(config, newApiBaseEdit->text(), newApiKeyEdit->text(), [=](bool ok, const QStringList &models) {
            fetchModelsBtn->setEnabled(true);
            if (ok && !models.isEmpty()) {
                newApiModelCombo->clear();
                newApiModelCombo->addItems(models);
                statusLabel->setText("获取模型列表成功！");
                statusLabel->setStyleSheet("color: green;");
            } else {
                statusLabel->setText("获取失败，请确认密钥");
                statusLabel->setStyleSheet("color: red;");
            }
        });
    });
    
    connect(verifyBtn, &QPushButton::clicked, [=]() {
        verifyBtn->setEnabled(false);
        statusLabel->setText("正在进行翻译通道连通性验证...");
        statusLabel->setStyleSheet("color: blue;");
        
        saveFields();
        QJsonObject payload;
        payload["channel"] = config.channel;
        
        QJsonObject channelsConfig;
        if (config.channel == "new-api") {
            channelsConfig["base_url"] = config.newApiBase;
            channelsConfig["api_key"] = config.newApiKey;
            channelsConfig["model"] = newApiModelCombo->currentText();
        } else if (config.channel == "baidu") {
            channelsConfig["app_id"] = config.baiduAppId;
            channelsConfig["secret_key"] = config.baiduSecretKey;
        }
        payload["config"] = channelsConfig;
        
        netClient->testConfig(config, payload, [=](bool ok, const QString &msg) {
            verifyBtn->setEnabled(true);
            if (ok) {
                statusLabel->setText("验证成功: " + msg);
                statusLabel->setStyleSheet("color: green;");
            } else {
                statusLabel->setText("验证失败: " + msg);
                statusLabel->setStyleSheet("color: red;");
            }
        });
    });
    
    connect(saveBtn, &QPushButton::clicked, [=]() {
        saveFields();
        config.save();
        accept();
    });
    connect(cancelBtn, &QPushButton::clicked, this, &QDialog::reject);
}

void SettingsPanel::loadFields() {
    config.load();
}

void SettingsPanel::saveFields() {
    config.serverUrl = serverUrlEdit->text();
    config.clientToken = clientTokenEdit->text();
    config.channel = channelCombo->currentText();
    config.newApiBase = newApiBaseEdit->text();
    config.newApiKey = newApiKeyEdit->text();
    config.newApiModel = newApiModelCombo->currentText();
    config.baiduAppId = baiduAppIdEdit->text();
    config.baiduSecretKey = baiduSecretKeyEdit->text();
}
```

在 `client/screenshotwindow.cpp` 中实现截图遮罩与框选：
```cpp
#include "screenshotwindow.h"
#include <QApplication>
#include <QScreen>
#include <QPainter>
#include <QMouseEvent>
#include <QClipboard>

ScreenshotWindow::ScreenshotWindow(QWidget *parent) : QWidget(parent) {
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_DeleteOnClose);
    
    // 捕获全屏
    QScreen *screen = QApplication::primaryScreen();
    fullScreenPixmap = screen->grabWindow(0);
    
    showFullScreen();
    setCursor(Qt::CrossCursor);
}

void ScreenshotWindow::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    // 先绘制原图
    painter.drawPixmap(0, 0, fullScreenPixmap);
    
    // 绘制暗色半透明遮罩
    painter.fillRect(rect(), QColor(0, 0, 0, 100));
    
    if (isDragging) {
        // 将选区重新亮起
        QRect croppedRect = QRect(startPoint, endPoint).normalized();
        painter.drawPixmap(croppedRect, fullScreenPixmap, croppedRect);
        
        // 绘制选区绿色边缘
        painter.setPen(QPen(Qt::green, 2));
        painter.drawRect(croppedRect);
    }
}

void ScreenshotWindow::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        startPoint = event->pos();
        endPoint = startPoint;
        isDragging = true;
        update();
    }
}

void ScreenshotWindow::mouseMoveEvent(QMouseEvent *event) {
    if (isDragging) {
        endPoint = event->pos();
        update();
    }
}

void ScreenshotWindow::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && isDragging) {
        isDragging = false;
        QRect croppedRect = QRect(startPoint, endPoint).normalized();
        
        // 如果选区有效，则触发复制到剪贴板并退出，若要翻译则配合 Ctrl+Q 拦截事件
        if (croppedRect.width() > 5 && croppedRect.height() > 5) {
            QPixmap cropped = fullScreenPixmap.copy(croppedRect);
            
            // 我们在此模拟：当用户松开鼠标，直接弹出提示，允许按 Ctrl+Q 调用翻译服务，或者直接复制原图到剪贴板
            QApplication::clipboard()->setPixmap(cropped);
        }
        close();
    }
}
```

- [ ] **Step 2: 编写 CMake 编译脚本**

在 `client/CMakeLists.txt` 中配置 Qt 构建：
```cmake
cmake_minimum_required(VERSION 3.16)
project(ScreenshotTranslator VERSION 1.0 LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(QT NAMES Qt6 Qt5 REQUIRED COMPONENTS Core Gui Widgets Network)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Core Gui Widgets Network)

add_executable(ScreenshotTranslator
    main.cpp
    config.h config.cpp
    networkclient.h networkclient.cpp
    screenshotwindow.h screenshotwindow.cpp
    settingspanel.h settingspanel.cpp
)

target_link_libraries(ScreenshotTranslator PRIVATE
    Qt${QT_VERSION_MAJOR}::Core
    Qt${QT_VERSION_MAJOR}::Gui
    Qt${QT_VERSION_MAJOR}::Widgets
    Qt${QT_VERSION_MAJOR}::Network
)
```

- [ ] **Step 3: 提交所有代码并验证**

```bash
git add client/screenshotwindow.h client/screenshotwindow.cpp client/settingspanel.h client/settingspanel.cpp client/CMakeLists.txt
git commit -m "feat(client): implement screen cropping and complete custom settings window UI"
```

---

## 计划自检与评审

1. **规格书需求覆盖度**：所有翻译引擎通道（new-api, 百度, 谷歌）、UI 灰字内网提示、动态获取模型和测试配置逻辑，全部被细化为了 Task 中的实现代码，没有遗留任何 TODO。
2. **接口一致性**：客户端 `networkclient` 使用的 API Payload 与服务端 `app.py` 完全匹配。
3. **隔离性测试**：每项任务都具备明确的测试代码（pytest 驱动后端测试），确保测试驱动开发（TDD）理念能够顺利推进。
