import abc
import requests
import json
import urllib.parse
import hashlib
import random
import logging
from concurrent.futures import ThreadPoolExecutor

logger = logging.getLogger(__name__)

# 使用全局共享的 requests Session 保持 Keep-Alive 长连接，免去每次 TLS 握手的开销
_shared_session = requests.Session()
# 适当调整连接池大小
adapter = requests.adapters.HTTPAdapter(pool_connections=10, pool_maxsize=20)
_shared_session.mount("http://", adapter)
_shared_session.mount("https://", adapter)

class BaseTranslator(abc.ABC):
    def __init__(self):
        self.session = _shared_session

    @abc.abstractmethod
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        pass

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        if not texts:
            return []
        with ThreadPoolExecutor(max_workers=8) as executor:
            futures = [executor.submit(self.translate, text, source_lang, target_lang) for text in texts]
            return [f.result() for f in futures]

class GoogleTranslator(BaseTranslator):
    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        url = f"https://translate.googleapis.com/translate_a/single?client=gtx&sl={source_lang}&tl={target_lang}&dt=t&q={urllib.parse.quote(text)}"
        response = self.session.get(url, timeout=5)
        if response.status_code == 200:
            try:
                res_json = response.json()
                if isinstance(res_json, list) and len(res_json) > 0 and isinstance(res_json[0], list):
                    return "".join([part[0] for part in res_json[0] if part[0]])
                raise ValueError("Unexpected JSON response structure from Google Translate")
            except (IndexError, TypeError, KeyError, ValueError) as e:
                raise Exception(f"Google translate response parsing failed: {e}")
        raise Exception(f"Google translate failed: status {response.status_code}")

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
        """
        优化：大厂同款批量翻译。将所有行合并成一次 POST 请求发送给 Google，
        利用 Google 对换行符 \n 的保留特性，在单次往返中取得全部翻译，
        最后进行行对齐。如果行数不符，平滑降级到并发线程池。
        """
        if not texts:
            return []
            
        # 1. 预处理文本，去掉行内换行防止干扰分行逻辑
        cleaned_texts = [t.replace('\n', ' ').strip() for t in texts]
        query = "\n".join(cleaned_texts)
        
        url = "https://translate.googleapis.com/translate_a/single"
        data = {
            "client": "gtx",
            "sl": source_lang,
            "tl": target_lang,
            "dt": "t",
            "q": query
        }
        
        try:
            # 使用 Post 请求，避免 Get 请求由于文本过长导致超长报错
            response = self.session.post(url, data=data, timeout=8)
            if response.status_code == 200:
                res_json = response.json()
                if isinstance(res_json, list) and len(res_json) > 0 and isinstance(res_json[0], list):
                    # 组合成完整的翻译文本
                    translated_full = "".join([part[0] for part in res_json[0] if part[0]])
                    # 按行切割
                    translated_lines = translated_full.splitlines()
                    
                    # 校验行数是否完全对应
                    if len(translated_lines) == len(texts):
                        return translated_lines
                    else:
                        logger.warning(
                            "[Google Batch] 翻译行数不匹配: 期望 %d 行，实际返回 %d 行。正在降级为线程池并发翻译...", 
                            len(texts), len(translated_lines)
                        )
        except Exception as e:
            logger.warning("[Google Batch] 批量翻译请求失败: %s。正在降级为线程池并发翻译...", e)
            
        # 2. 降级兜底：使用基类的多线程并发请求，保证稳定性
        return super().translate_batch(texts, source_lang, target_lang)

class LLMTranslator(BaseTranslator):
    def __init__(self, base_url: str, api_key: str, model: str):
        super().__init__()
        self.base_url = base_url.rstrip('/')
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
                {"role": "system", "content": "You are a translation assistant. Translate the following text into Simplified Chinese. Output ONLY the translated text, do not include any commentary, explanations, or quotes."},
                {"role": "user", "content": text}
            ],
            "temperature": 0.3
        }
        res = self.session.post(f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=10)
        if res.status_code == 200:
            return res.json()["choices"][0]["message"]["content"].strip()
        raise Exception(f"LLM translation failed: {res.text}")

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
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
            res = self.session.post(f"{self.base_url}/v1/chat/completions", headers=headers, json=payload, timeout=12)
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
        super().__init__()
        self.app_id = app_id
        self.secret_key = secret_key

    def translate(self, text: str, source_lang: str, target_lang: str) -> str:
        salt = str(random.randint(32768, 65536))
        sign_str = self.app_id + text + salt + self.secret_key
        sign = hashlib.md5(sign_str.encode('utf-8')).hexdigest()
        
        from_lang = "auto" if source_lang == "auto" else source_lang
        to_lang = "zh" if target_lang == "zh" else target_lang
        
        url = f"https://fanyi-api.baidu.com/api/trans/vip/translate?q={urllib.parse.quote(text)}&from={from_lang}&to={to_lang}&appid={self.app_id}&salt={salt}&sign={sign}"
        res = self.session.get(url, timeout=5)
        if res.status_code == 200:
            res_json = res.json()
            if "error_code" in res_json:
                raise Exception(f"Baidu translation API error: {res_json['error_msg']}")
            return "".join([item["dst"] for item in res_json["trans_result"]])
        raise Exception(f"Baidu request failed: status {res.status_code}")

    def translate_batch(self, texts: list[str], source_lang: str, target_lang: str, stats_ref: dict = None) -> list[str]:
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
            res = self.session.post(url, data=data, timeout=8)
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

