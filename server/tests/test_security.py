import pytest

from security import normalize_public_base_url


def test_deepl_public_host_allows_proxy_dns_reserved_ip(monkeypatch):
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(None, None, None, None, ("198.18.0.127", 0))],
    )

    assert normalize_public_base_url("https://api-free.deepl.com") == "https://api-free.deepl.com"


def test_public_url_still_rejects_private_dns(monkeypatch):
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(None, None, None, None, ("127.0.0.1", 0))],
    )

    with pytest.raises(ValueError):
        normalize_public_base_url("https://example.com")
