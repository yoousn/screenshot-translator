import abc
import requests
import json
import urllib.parse
import hashlib
import random
import logging

logger = logging.getLogger(__name__)

class BaseTranslator(abc.ABC):
    @abc.abstractmethod
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        pass

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str) -> list[str]:
        from concurrent.futures import ThreadPoolExecutor
        if not texts:
            return []
        with ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(self.translate, text, source_lang, target_lang) for text in texts]
            return [f.result() for f in futures]

class GoogleTranslator(BaseTranslator):
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        # 使用 Google Web 翻译免密接口
        url = f"https://translate.googleapis.com/translate_a/single?client=gtx&sl={source_lang}&tl={target_lang}&dt=t&q={urllib.parse.quote(text)}"
        response = requests.get(url, timeout=5)
        if response.status_code == 200:
            try:
                res_json = response.json()
                if isinstance(res_json, list) and len(res_json) > 0 and isinstance(res_json[0], list):
                    return "".join([part[0] for part in res_json[0] if part[0]])
                raise ValueError("Unexpected JSON response structure from Google Translate")
            except (IndexError, TypeError, KeyError, ValueError) as e:
                raise Exception(f"Google translate response parsing failed: {e}")
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

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str) -> list[str]:
        if not texts:
            return []
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        prompt = (
            "You are a translation assistant. Translate the following list of texts into Simplified Chinese.\n"
            "The input is a JSON array of strings. You must return a JSON array of strings containing the translations in the same order.\n"
            "Respond ONLY with a valid JSON array of strings. Do not wrap it in markdown code blocks like ```json."
        )
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": prompt},
                {"role": "user", "content": json.dumps(texts, ensure_ascii=False)}
            ],
            "temperature": 0.2
        }
        try:
            res = requests.post(f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=12)
            if res.status_code == 200:
                content = res.json()["choices"][0]["message"]["content"].strip()
                if content.startswith("```"):
                    lines = content.splitlines()
                    if lines[0].startswith("```"):
                        lines = lines[1:]
                    if lines and lines[-1].startswith("```"):
                        lines = lines[:-1]
                    content = "\n".join(lines).strip()
                translated = json.loads(content)
                if isinstance(translated, list) and len(translated) == len(texts):
                    return [str(x) for x in translated]
        except Exception as e:
            logger.warning("LLM batch translation failed: %s", e)
        return super().translate_batch(texts, source_lang, target_lang)

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

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str) -> list[str]:
        if not texts:
            return []
        cleaned_texts = [t.replace('\n', ' ').strip() for t in texts]
        query = "\n".join(cleaned_texts)
        
        salt = str(random.randint(32768, 65536))
        sign_str = self.app_id + query + salt + self.secret_key
        sign = hashlib.md5(sign_str.encode('utf-8')).hexdigest()
        
        from_lang = "auto" if source_lang == "auto" else source_lang
        to_lang = "zh" if target_lang == "zh" else target_lang
        
        url = "https://fanyi-api.baidu.com/api/trans/vip/translate"
        data = {
            "q": query,
            "from": from_lang,
            "to": to_lang,
            "appid": self.app_id,
            "salt": salt,
            "sign": sign
        }
        try:
            res = requests.post(url, data=data, timeout=8)
            if res.status_code == 200:
                res_json = res.json()
                if "error_code" in res_json:
                    raise Exception(f"Baidu translation API error: {res_json['error_msg']}")
                trans_result = res_json.get("trans_result", [])
                if len(trans_result) == len(texts):
                    return [item["dst"] for item in trans_result]
        except Exception as e:
            logger.warning("Baidu batch translation failed: %s", e)
        return super().translate_batch(texts, source_lang, target_lang)
