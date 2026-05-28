import pytest
from fastapi.testclient import TestClient
import sys
import os
from unittest.mock import patch

# 确保 PYTHONPATH 能找到 app.py
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from app import app
from config import load_server_config

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

def test_translate_text_success():
    cfg = load_server_config()
    token = cfg["client_token"]
    
    with patch("translator.GoogleTranslator.translate") as mock_translate:
        mock_translate.side_effect = lambda text, *args, **kwargs: f"Translat: {text}"
        
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
        assert data["translations"][0] == "Translat: Hello"
        assert data["translations"][1] == "Translat: World"
