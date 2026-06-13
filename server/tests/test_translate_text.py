import pytest
from fastapi.testclient import TestClient
import sys
import os
from unittest.mock import patch

# 确保 PYTHONPATH 能找到 app.py
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from app import app
from config import load_server_config
from translator import GoogleTranslator, detect_source_lang_hint, lookup_short_ui_glossary, has_likely_non_english_latin_text, TRANSLATION_GLOSSARY

client = TestClient(app)

def test_translate_text_unauthorized():
    response = client.post("/api/translate_text", json={"blocks": []})
    assert response.status_code == 401

def test_translate_text_empty_blocks():
    cfg = load_server_config()
    token = cfg["client_token"]
    response = client.post(
        "/api/translate_text",
        headers={"X-API-Key": token},
        json={"blocks": [], "source_lang": "auto", "target_lang": "zh"}
    )
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "success"
    assert data["translations"] == []
    assert data["timings"]["blocks"] == 0
    assert data["timings"]["provider_misses"] == 0


def test_translate_text_forwards_v6_technical_text_override():
    cfg = load_server_config()
    token = cfg["client_token"]
    captured = {}
    path = "C:\\Users\\ysn\\Desktop\\app.exe"

    class FakeTranslator:
        def translate_batch(self, texts, source_lang, target_lang, stats_ref, **kwargs):
            captured["texts"] = texts
            captured["force_translate_technical_text"] = kwargs.get("force_translate_technical_text")
            return [f"应用程序路径：{texts[0]}"]

    with patch("app.get_active_translator", return_value=FakeTranslator()):
        response = client.post(
            "/api/translate_text",
            headers={"X-API-Key": token},
            json={
                "blocks": [{"text": path, "confidence": 0.99, "box": [[0, 0], [200, 0], [200, 20], [0, 20]]}],
                "source_lang": "en",
                "target_lang": "zh",
                "force_translate_technical_text": True,
            },
        )

    assert response.status_code == 200
    assert response.json()["translations"] == [f"应用程序路径：{path}"]
    assert captured["texts"] == [path]
    assert captured["force_translate_technical_text"] is True


def test_translate_text_batch_failure_returns_aligned_blank_translations():
    cfg = load_server_config()
    token = cfg["client_token"]

    class FakeTranslator:
        def translate_batch(self, *args, **kwargs):
            raise RuntimeError("force batch failure")

        def translate(self, text, *args, **kwargs):
            raise AssertionError("endpoint must not perform slow per-item fallback")

    with patch("app.get_active_translator", return_value=FakeTranslator()):
        
        payload = {
            "blocks": [
                {"text": "Hello", "confidence": 0.9, "box": [[0,0],[10,0],[10,5],[0,5]]},
                {"text": "World", "confidence": 0.8, "box": [[20,0],[30,0],[30,5],[20,5]]}
            ],
            "source_lang": "en",
            "target_lang": "zh"
        }
        response = client.post(
            "/api/translate_text",
            headers={"X-API-Key": token},
            json=payload
        )
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "success"
        assert len(data["translations"]) == 2
        assert data["translations"] == ["", ""]
        assert data["timings"]["blocks"] == 2
        assert data["timings"]["provider_misses"] == 2
        assert data["timings"]["provider_failures"] == 2
        assert data["timings"]["total_ms"] >= 0

def test_translate_text_fallback_failure_returns_blank_translation():
    cfg = load_server_config()
    token = cfg["client_token"]

    class BrokenTranslator:
        def translate_batch(self, *args, **kwargs):
            raise RuntimeError("batch down")

        def translate(self, *args, **kwargs):
            raise RuntimeError("single down")

    payload = {
        "blocks": [
            {"text": "Open preview", "confidence": 0.9, "box": [[0,0],[10,0],[10,5],[0,5]]},
        ],
        "source_lang": "en",
        "target_lang": "zh"
    }
    with patch("app.get_active_translator", return_value=BrokenTranslator()):
        response = client.post(
            "/api/translate_text",
            headers={"X-API-Key": token},
            json=payload
        )
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "success"
    assert data["translations"] == [""]
    assert data["timings"]["blocks"] == 1
    assert data["timings"]["provider_misses"] == 1
    assert data["timings"]["provider_failures"] == 1


def test_translate_text_provider_error_is_redacted():
    cfg = load_server_config()
    token = cfg["client_token"]

    class LeakyTranslator:
        def translate_batch(self, *args, **kwargs):
            raise RuntimeError("x-api-key: sk-secret-token")

    payload = {
        "blocks": [
            {"text": "Open preview", "confidence": 0.9, "box": [[0,0],[10,0],[10,5],[0,5]]},
        ],
        "source_lang": "en",
        "target_lang": "zh"
    }
    with patch("app.get_active_translator", return_value=LeakyTranslator()):
        response = client.post(
            "/api/translate_text",
            headers={"X-API-Key": token},
            json=payload
        )
    assert response.status_code == 200
    provider_error = response.json()["provider_error"]
    assert "sk-secret-token" not in provider_error
    assert "***REDACTED***" in provider_error


def test_translate_text_mismatched_provider_result_returns_aligned_blank_translations():
    cfg = load_server_config()
    token = cfg["client_token"]

    class MisalignedTranslator:
        def translate_batch(self, *args, **kwargs):
            return ["only one"]

    payload = {
        "blocks": [
            {"text": "Open", "confidence": 0.9, "box": [[0,0],[10,0],[10,5],[0,5]]},
            {"text": "Save", "confidence": 0.9, "box": [[20,0],[30,0],[30,5],[20,5]]},
        ],
        "source_lang": "en",
        "target_lang": "zh",
    }
    with patch("app.get_active_translator", return_value=MisalignedTranslator()):
        response = client.post(
            "/api/translate_text",
            headers={"X-API-Key": token},
            json=payload,
        )
    assert response.status_code == 200
    data = response.json()
    assert data["translations"] == ["", ""]
    assert "provider_error" in data
    assert data["timings"]["blocks"] == 2

def test_google_translator_uses_script_source_hint_for_korean():
    class FakeResponse:
        status_code = 200

        def json(self):
            return [[["保存您的文件"]]]

    captured = {}

    class FakeSession:
        def get(self, url, timeout, **_kwargs):
            captured["url"] = url
            return FakeResponse()

    translator = GoogleTranslator()
    translator.session = FakeSession()
    assert detect_source_lang_hint("파일을 저장하세요", "auto") == "ko"
    assert translator.translate("파일을 저장하세요", "auto", "zh") == "保存您的文件"
    assert "sl=ko" in captured["url"]


def test_short_ui_glossary_prefers_interface_meaning():
    assert TRANSLATION_GLOSSARY["version"] != "fallback"
    assert TRANSLATION_GLOSSARY["zh"]["ui"]["save"] == "保存"
    assert lookup_short_ui_glossary("Save", "en", "zh") == "保存"
    assert lookup_short_ui_glossary("Open preview", "auto", "zh-CN") == "打开预览"
    assert lookup_short_ui_glossary("Save", "ja", "zh") is None


def test_latin_non_english_text_uses_auto_source_hint():
    assert has_likely_non_english_latin_text("Ouvrir l'aperçu avant d'enregistrer")
    assert has_likely_non_english_latin_text("Abrir vista previa antes de guardar")
    assert detect_source_lang_hint("Ouvrir l'aperçu avant d'enregistrer", "en") == "auto"
    assert detect_source_lang_hint("Abrir vista previa antes de guardar", "en") == "auto"
