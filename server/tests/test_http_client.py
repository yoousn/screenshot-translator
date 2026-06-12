from requests.adapters import HTTPAdapter

from http_client import get_official_translation_session, get_public_session, get_relay_session
from safe_transport import SSRFSafeAdapter


def test_official_translation_session_uses_environment_proxy_without_pinning():
    session = get_official_translation_session()

    assert session.trust_env is True
    assert isinstance(session.get_adapter("https://"), HTTPAdapter)
    assert not isinstance(session.get_adapter("https://"), SSRFSafeAdapter)


def test_user_configured_url_sessions_keep_pinned_transport():
    public_session = get_public_session()
    relay_session = get_relay_session()

    assert public_session.trust_env is False
    assert relay_session.trust_env is False
    assert isinstance(public_session.get_adapter("https://"), SSRFSafeAdapter)
    assert isinstance(relay_session.get_adapter("https://"), SSRFSafeAdapter)
