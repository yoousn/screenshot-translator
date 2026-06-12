import requests

from safe_transport import SSRFSafeAdapter


def _make_pinned_session(*, allow_private: bool) -> requests.Session:
    session = requests.Session()
    session.trust_env = False
    adapter = SSRFSafeAdapter(
        allow_private=allow_private,
        pool_connections=10,
        pool_maxsize=20,
    )
    session.mount("http://", adapter)
    session.mount("https://", adapter)
    return session


def _make_official_translation_session() -> requests.Session:
    session = requests.Session()
    session.trust_env = True
    adapter = requests.adapters.HTTPAdapter(pool_connections=10, pool_maxsize=20)
    session.mount("http://", adapter)
    session.mount("https://", adapter)
    return session


_OFFICIAL_TRANSLATION_SESSION = _make_official_translation_session()
_PUBLIC_SESSION = _make_pinned_session(allow_private=False)
_RELAY_SESSION = _make_pinned_session(allow_private=True)


def get_official_translation_session() -> requests.Session:
    return _OFFICIAL_TRANSLATION_SESSION


def get_public_session() -> requests.Session:
    return _PUBLIC_SESSION


def get_relay_session() -> requests.Session:
    return _RELAY_SESSION
