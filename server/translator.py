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
        # 如果 base_url 不含 http/https，在此补足
        if not self.base_url.startswith("http://") and not self.base_url.startswith("https://"):
            self.base_url = "https://" + self.base_url
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
