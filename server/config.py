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
                    "base_url": "api.yousn.me",
                    "api_key": "sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb",
                    "model": "gemini-3.5-flash"
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
        try:
            cfg = yaml.safe_load(f)
            if not cfg or not isinstance(cfg, dict):
                raise ValueError("Empty or invalid config structure")
            return cfg
        except Exception:
            # 如果解析失败，重新生成默认配置
            default = {
                "client_token": "ysn-screenshot-translator-token-666",
                "active_channel": "google",
                "channels": {
                    "new-api": {
                        "base_url": "api.yousn.me",
                        "api_key": "sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb",
                        "model": "gemini-3.5-flash"
                    },
                    "baidu": {
                        "app_id": "",
                        "secret_key": ""
                    }
                }
            }
            save_server_config(default)
            return default

def save_server_config(cfg):
    os.makedirs(os.path.dirname(CONFIG_PATH), exist_ok=True)
    with open(CONFIG_PATH, 'w', encoding='utf-8') as f:
        yaml.safe_dump(cfg, f, allow_unicode=True)
