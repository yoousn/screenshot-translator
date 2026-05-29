import yaml
import os
import shutil
import logging
import secrets

logger = logging.getLogger(__name__)

CONFIG_PATH = os.path.expanduser("~/.screenshot-translator/config.yaml")

def _generate_token():
    """Generate a cryptographically secure random token."""
    return secrets.token_urlsafe(32)

def _default_config():
    return {
        "client_token": _generate_token(),
        "active_channel": "google",
        "debug_trace": False,
        "ocr_max_side": 1280,
        "ocr_cache_enabled": True,
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
        logger.info("[Security] 首次启动，已自动生成随机 client_token 并写入配置文件。")
        logger.info("[Security] 请在客户端填入此 token: %s", default["client_token"])
        return _apply_env_overrides(default)
    
    try:
        with open(CONFIG_PATH, 'r', encoding='utf-8') as f:
            cfg = yaml.safe_load(f)
            if not cfg or not isinstance(cfg, dict):
                raise ValueError("Empty or invalid config structure")
            return _apply_env_overrides(cfg)
    except Exception as e:
        logger.warning("Config parsing failed: %s. Backing up old config and regenerating default.", e)
        try:
            shutil.copy2(CONFIG_PATH, CONFIG_PATH + ".bak")
        except Exception as backup_err:
            logger.error("Failed to create config backup: %s", backup_err)
        default = _default_config()
        save_server_config(default)
        return _apply_env_overrides(default)

def _apply_env_overrides(cfg: dict) -> dict:
    """Allow SS_TRANSLATOR_TOKEN env var to override the persisted token at runtime."""
    env_token = os.environ.get("SS_TRANSLATOR_TOKEN")
    if env_token:
        cfg["client_token"] = env_token
    return cfg

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
