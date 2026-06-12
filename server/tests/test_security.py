import pytest
from urllib3.connection import HTTPSConnection

from safe_transport import _connect_pinned_address
from security import normalize_public_base_url, normalize_relay_base_url, redact, request_public_url, request_relay_url, resolve_safe_ip


class FakeSocket:
    connected = []
    options = []

    def __init__(self, *args):
        self.args = args
        self.timeout = None
        self.closed = False

    def setsockopt(self, *args):
        self.options.append(args)

    def settimeout(self, timeout):
        self.timeout = timeout

    def bind(self, *_args):
        pass

    def connect(self, sockaddr):
        self.connected.append(sockaddr)

    def close(self):
        self.closed = True


class RedirectResponse:
    status_code = 302
    headers = {"Location": "http://169.254.169.254/latest/meta-data"}


class RedirectSession:
    def __init__(self):
        self.calls = 0

    def request(self, *_args, **_kwargs):
        self.calls += 1
        return RedirectResponse()


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


@pytest.mark.parametrize(
    "ip",
    [
        "127.0.0.1",
        "10.0.0.5",
        "172.16.0.10",
        "192.168.1.20",
        "169.254.169.254",
        "::1",
        "fc00::1",
        "fe80::1",
    ],
)
def test_public_url_rejects_blocked_address_ranges(monkeypatch, ip):
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(2, 1, 6, "", (ip, args[1] if len(args) > 1 else 443))],
    )

    with pytest.raises(ValueError):
        normalize_public_base_url("https://blocked.example")


def test_resolve_safe_ip_skips_private_and_returns_public(monkeypatch):
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [
            (None, None, None, None, ("10.0.0.5", 0)),
            (None, None, None, None, ("93.184.216.34", 0)),
        ],
    )

    assert resolve_safe_ip("example.com") == "93.184.216.34"


def test_pinned_https_connection_keeps_host_and_connects_safe_ip(monkeypatch):
    calls = []
    FakeSocket.connected = []
    FakeSocket.options = []

    def fake_getaddrinfo(host, port, *args, **kwargs):
        calls.append((host, port))
        return [
            (2, 1, 6, "", ("93.184.216.34", port)),
            (2, 1, 6, "", ("10.0.0.5", port)),
        ]

    monkeypatch.setattr("security.socket.getaddrinfo", fake_getaddrinfo)
    monkeypatch.setattr("safe_transport.socket.socket", FakeSocket)

    conn = HTTPSConnection("example.com", port=443, timeout=1)
    sock = _connect_pinned_address(conn, allow_private=False)

    assert isinstance(sock, FakeSocket)
    assert conn.host == "example.com"
    assert conn._dns_host == "example.com"
    assert calls == [("example.com", 443)]
    assert FakeSocket.connected == [("93.184.216.34", 443)]


def test_pinned_connection_falls_back_to_host_without_dns_host(monkeypatch):
    calls = []
    FakeSocket.connected = []

    class MinimalConnection:
        host = "fallback.example"
        port = 443
        timeout = 1
        source_address = None
        socket_options = []

    def fake_getaddrinfo(host, port, *args, **kwargs):
        calls.append((host, port))
        return [(2, 1, 6, "", ("93.184.216.34", port))]

    monkeypatch.setattr("security.socket.getaddrinfo", fake_getaddrinfo)
    monkeypatch.setattr("safe_transport.socket.socket", FakeSocket)

    _connect_pinned_address(MinimalConnection(), allow_private=False)

    assert calls == [("fallback.example", 443)]
    assert FakeSocket.connected == [("93.184.216.34", 443)]


def test_pinned_connection_applies_socket_options(monkeypatch):
    FakeSocket.connected = []
    FakeSocket.options = []
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(2, 1, 6, "", ("93.184.216.34", args[1]))],
    )
    monkeypatch.setattr("safe_transport.socket.socket", FakeSocket)

    conn = HTTPSConnection("example.com", port=443, timeout=1)
    conn.socket_options = [(1, 2, 3)]
    _connect_pinned_address(conn, allow_private=False)

    assert FakeSocket.options == [(1, 2, 3)]


def test_pinned_public_connection_rejects_private_address(monkeypatch):
    FakeSocket.connected = []
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(2, 1, 6, "", ("127.0.0.1", args[1]))],
    )
    monkeypatch.setattr("safe_transport.socket.socket", FakeSocket)

    conn = HTTPSConnection("example.com", port=443, timeout=1)
    with pytest.raises(Exception):
        _connect_pinned_address(conn, allow_private=False)
    assert FakeSocket.connected == []


def test_pinned_relay_connection_allows_private_address(monkeypatch):
    FakeSocket.connected = []
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(2, 1, 6, "", ("127.0.0.1", args[1]))],
    )
    monkeypatch.setattr("safe_transport.socket.socket", FakeSocket)

    conn = HTTPSConnection("local-relay.test", port=8318, timeout=1)
    _connect_pinned_address(conn, allow_private=True)
    assert FakeSocket.connected == [("127.0.0.1", 8318)]


def test_public_redirect_to_link_local_is_rejected(monkeypatch):
    def fake_getaddrinfo(host, port, *args, **kwargs):
        if host == "example.com":
            return [(2, 1, 6, "", ("93.184.216.34", port or 80))]
        return [(2, 1, 6, "", ("169.254.169.254", port or 80))]

    monkeypatch.setattr("security.socket.getaddrinfo", fake_getaddrinfo)
    session = RedirectSession()

    with pytest.raises(ValueError):
        request_public_url(session, "GET", "https://example.com")
    assert session.calls == 1


def test_public_rejects_private_url_but_relay_allows_same_url(monkeypatch):
    monkeypatch.setattr(
        "security.socket.getaddrinfo",
        lambda *args, **kwargs: [(2, 1, 6, "", ("127.0.0.1", args[1] if len(args) > 1 else 8318))],
    )

    with pytest.raises(ValueError):
        request_public_url(RedirectSession(), "GET", "http://127.0.0.1:8318")

    class OkResponse:
        status_code = 200
        headers = {}

    class OkSession:
        def __init__(self):
            self.calls = 0

        def request(self, *_args, **_kwargs):
            self.calls += 1
            return OkResponse()

    session = OkSession()
    assert normalize_relay_base_url("http://127.0.0.1:8318") == "http://127.0.0.1:8318"
    assert request_relay_url(session, "GET", "http://127.0.0.1:8318").status_code == 200
    assert session.calls == 1


def test_redact_masks_tokens_and_api_keys():
    text = "client_token=abc123456 x-api-key: sk-secret Authorization: Bearer bearer-secret"
    redacted = redact(text)
    assert "abc123456" not in redacted
    assert "sk-secret" not in redacted
    assert "bearer-secret" not in redacted
    assert "***REDACTED***" in redacted
