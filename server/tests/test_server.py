import pytest
from fastapi.testclient import TestClient
import sys
import os
from unittest.mock import patch, MagicMock

# 确保 PYTHONPATH 能找到 app.py
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from app import app
from config import load_server_config

client = TestClient(app)

def test_health_exposes_translation_metadata():
    res = client.get("/api/health")
    assert res.status_code == 200
    data = res.json()
    assert data["status"] == "ok"
    assert data["translation"]["glossary_loaded"] is True
    assert data["translation"]["glossary_version"] != "fallback"
    assert data["translation"]["quality_flags"]["latin_non_english_auto_source"] is True

def test_fetch_models_unauthorized():
    res = client.post("/api/config/fetch_models", json={"base_url": "api.yousn.me", "api_key": "sk-xxx"})
    assert res.status_code == 401

def test_config_test_unauthorized():
    res = client.post("/api/config/test", json={"channel": "google"})
    assert res.status_code == 401

def test_fetch_models_success():
    cfg = load_server_config()
    token = cfg["client_token"]
    
    with patch("app.request_public_url") as mock_request, patch("app.normalize_public_base_url", return_value="https://api.yousn.me"):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": [
                {"id": "gemini-1.5-pro"},
                {"id": "gemini-1.5-flash"},
                {"id": "gpt-4o"}
            ]
        }
        mock_request.return_value = mock_response
        
        res = client.post(
            "/api/config/fetch_models",
            headers={"X-API-Key": token},
            json={"base_url": "api.yousn.me", "api_key": "sk-xxx"}
        )
        assert res.status_code == 200
        data = res.json()
        assert data["status"] == "success"
        assert "gemini-1.5-flash" in data["models"]

def test_fetch_models_rejects_private_url():
    cfg = load_server_config()
    token = cfg["client_token"]

    res = client.post(
        "/api/config/fetch_models",
        headers={"X-API-Key": token},
        json={"base_url": "http://127.0.0.1:3001", "api_key": "sk-xxx"}
    )
    assert res.status_code == 200
    data = res.json()
    assert data["status"] == "failed"
    assert "请求地址" in data["error"]

def test_config_test_google_success():
    cfg = load_server_config()
    token = cfg["client_token"]
    
    with patch("translator.GoogleTranslator.translate") as mock_translate:
        mock_translate.return_value = "测试连接成功"
        
        res = client.post(
            "/api/config/test",
            headers={"X-API-Key": token},
            json={"channel": "google", "config": {}}
        )
        assert res.status_code == 200
        data = res.json()
        assert data["status"] == "success"
        assert data["result"] == "测试连接成功"

def test_config_save_google_success():
    cfg = load_server_config()
    token = cfg["client_token"]

    res = client.post(
        "/api/config/save",
        headers={"X-API-Key": token},
        json={"channel": "google", "config": {}}
    )
    assert res.status_code == 200
    data = res.json()
    assert data["status"] == "success"
    assert data["active_channel"] == "google"


def test_current_config_exposes_translation_metadata():
    cfg = load_server_config()
    token = cfg["client_token"]

    res = client.get("/api/config/current", headers={"X-API-Key": token})
    assert res.status_code == 200
    data = res.json()
    assert data["status"] == "success"
    assert data["translation"]["glossary_loaded"] is True
    assert data["translation"]["glossary_terms"] > 0

