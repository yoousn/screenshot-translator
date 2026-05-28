import yaml
import os
import shutil
import logging

logger = logging.getLogger(__name__)

CONFIG_PATH = os.path.expanduser("~/.screenshot-translator/config.yaml")

def _default_config():
    return {
        "client_token": "ysn-screenshot-translator-token-666",
        "active_channel": "google",
        "debug_trace": True,
        "channels": {
            "new-api": {
                "base_url": "api.yousn.me",
                "api_key": os.environ.get("SS_TRANSLATOR_API_KEY", ""),
                "model": "gemini-3.5-flash"
            },
            "baidu": {
                "app_id": "",
                "secret_key": ""
            }
        }
    }

def load_server_config():
    if not os.path.exists(CONFIG_PATH):
        os.makedirs(os.path.dirname(CONFIG_PATH), exist_ok=True)
        default = _default_config()
        save_server_config(default)
        return default
    
    try:
        with open(CONFIG_PATH, 'r', encoding='utf-8') as f:
            cfg = yaml.safe_load(f)
            if not cfg or not isinstance(cfg, dict):
                raise ValueError("Empty or invalid config structure")
            return cfg
    except Exception as e:
        logger.warning("Config parsing failed: %s. Backing up old config and regenerating default.", e)
        try:
            shutil.copy2(CONFIG_PATH, CONFIG_PATH + ".bak")
        except Exception as backup_err:
            logger.error("Failed to create config backup: %s", backup_err)
        default = _default_config()
        save_server_config(default)
        return default

def save_server_config(cfg):
    dir_name = os.path.dirname(CONFIG_PATH)
    os.makedirs(dir_name, exist_ok=True)
    tmp_path = CONFIG_PATH + ".tmp"
    with open(tmp_path, 'w', encoding='utf-8') as f:
        yaml.safe_dump(cfg, f, allow_unicode=True)
        f.flush()
        try:
            os.fsync(f.fileno())
        except OSError:
            pass
    os.replace(tmp_path, CONFIG_PATH)
    try:
        os.chmod(CONFIG_PATH, 0o600)
    except Exception:
        pass
