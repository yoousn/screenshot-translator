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

def test_api_unauthorized():
    res = client.post("/api/translate", files={"image": ("fake.png", b"fake-data", "image/png")})
    assert res.status_code == 401

def test_fetch_models_unauthorized():
    res = client.post("/api/config/fetch_models", json={"base_url": "api.yousn.me", "api_key": "sk-xxx"})
    assert res.status_code == 401

def test_config_test_unauthorized():
    res = client.post("/api/config/test", json={"channel": "google"})
    assert res.status_code == 401

def test_fetch_models_success():
    cfg = load_server_config()
    token = cfg["client_token"]
    
    with patch("requests.get") as mock_get, patch("app._validate_url", return_value=True):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "data": [
                {"id": "gemini-1.5-pro"},
                {"id": "gemini-1.5-flash"},
                {"id": "gpt-4o"}
            ]
        }
        mock_get.return_value = mock_response
        
        res = client.post(
            "/api/config/fetch_models",
            headers={"X-API-Key": token},
            json={"base_url": "api.yousn.me", "api_key": "sk-xxx"}
        )
        assert res.status_code == 200
        data = res.json()
        assert data["status"] == "success"
        assert "gemini-1.5-flash" in data["models"]

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
