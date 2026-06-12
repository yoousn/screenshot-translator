import ipaddress
import logging
import re
import socket
import urllib.parse


SECRET_PATTERN = re.compile(
    r"(authorization|x-api-key|client[_\s-]*token|token|api[_-]?key|secret[_-]?key)\s*[:=]\s*([^\s,;]+)|"
    r"(bearer|deepl-auth-key)\s+([A-Za-z0-9._\-]+)",
    re.IGNORECASE,
)
AUTH_BEARER_PATTERN = re.compile(
    r"(authorization\s*[:=]\s*bearer\s+)([^\s,;]+)",
    re.IGNORECASE,
)


def redact(text: object) -> str:
    value = str(text)
    value = AUTH_BEARER_PATTERN.sub(lambda match: f"{match.group(1)}***REDACTED***", value)
    return SECRET_PATTERN.sub(lambda match: f"{match.group(1) or match.group(3)}=***REDACTED***", value)


class RedactFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        record.msg = redact(record.getMessage())
        record.args = ()
        return True


_ORIGINAL_LOG_RECORD_FACTORY = logging.getLogRecordFactory()
_REDACTION_FACTORY_INSTALLED = False


def install_redaction_filter() -> None:
    global _REDACTION_FACTORY_INSTALLED
    if not _REDACTION_FACTORY_INSTALLED:

        def redacting_factory(*args, **kwargs):
            record = _ORIGINAL_LOG_RECORD_FACTORY(*args, **kwargs)
            record.msg = redact(record.getMessage())
            record.args = ()
            return record

        logging.setLogRecordFactory(redacting_factory)
        _REDACTION_FACTORY_INSTALLED = True

    root = logging.getLogger()
    if any(isinstance(item, RedactFilter) for item in root.filters):
        return
    root.addFilter(RedactFilter())


TRUSTED_PUBLIC_DNS_BYPASS_HOSTS = {
    "api-free.deepl.com",
    "api.deepl.com",
}


def _is_blocked_ip(ip: ipaddress._BaseAddress) -> bool:
    return (
        ip.is_private
        or ip.is_loopback
        or ip.is_link_local
        or ip.is_reserved
        or ip.is_unspecified
        or ip.is_multicast
    )


def iter_safe_addresses(host: str, port: int, *, allow_private: bool = False):
    try:
        addr_info = socket.getaddrinfo(host, port, type=socket.SOCK_STREAM, proto=socket.IPPROTO_TCP)
    except OSError as exc:
        raise ValueError("请求地址无法解析") from exc

    for family, socktype, proto, _canonname, sockaddr in addr_info:
        ip_str = sockaddr[0].split("%", 1)[0]
        ip = ipaddress.ip_address(ip_str)
        if _is_blocked_ip(ip) and not allow_private:
            continue
        yield family, socktype, proto, sockaddr, ip_str


def resolve_safe_ip(host: str, *, allow_private: bool = False) -> str:
    """Resolve a host and return the first IP allowed by the SSRF policy."""
    for *_prefix, ip_str in iter_safe_addresses(host, 0, allow_private=allow_private):
        return ip_str
    raise ValueError("请求地址不合法（无可用公网 IP）")


def normalize_base_url(url: str, *, allow_private: bool = False) -> str:
    base_url = (url or "").strip().rstrip("/")
    if not base_url:
        raise ValueError("中转地址不能为空")
    if not base_url.startswith(("http://", "https://")):
        base_url = "https://" + base_url

    parsed = urllib.parse.urlparse(base_url)
    trusted_public_host = (
        not allow_private
        and parsed.hostname is not None
        and parsed.hostname.lower() in TRUSTED_PUBLIC_DNS_BYPASS_HOSTS
    )
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise ValueError("请求地址不合法")

    if trusted_public_host:
        resolve_safe_ip(parsed.hostname, allow_private=True)
    else:
        resolve_safe_ip(parsed.hostname, allow_private=allow_private)

    return base_url


def normalize_public_base_url(url: str) -> str:
    return normalize_base_url(url, allow_private=False)


def normalize_relay_base_url(url: str) -> str:
    return normalize_base_url(url, allow_private=True)


def request_validated_url(
    session,
    method: str,
    url: str,
    *,
    allow_private: bool = False,
    max_redirects: int = 3,
    **kwargs,
):
    """Request a URL while validating every redirect target."""
    current_url = normalize_base_url(url, allow_private=allow_private)
    redirects = 0
    while True:
        response = session.request(method, current_url, allow_redirects=False, **kwargs)
        if response.status_code not in {301, 302, 303, 307, 308}:
            return response
        if redirects >= max_redirects:
            raise ValueError("重定向次数过多")
        location = response.headers.get("Location")
        if not location:
            return response
        current_url = normalize_base_url(urllib.parse.urljoin(current_url, location), allow_private=allow_private)
        if response.status_code == 303:
            method = "GET"
            kwargs.pop("data", None)
            kwargs.pop("json", None)
        redirects += 1


def request_public_url(session, method: str, url: str, *, max_redirects: int = 3, **kwargs):
    """Request a public URL while validating every redirect target."""
    return request_validated_url(session, method, url, allow_private=False, max_redirects=max_redirects, **kwargs)


def request_relay_url(session, method: str, url: str, *, max_redirects: int = 3, **kwargs):
    """Request a user-configured relay URL. Self-hosted LAN relays are allowed."""
    return request_validated_url(session, method, url, allow_private=True, max_redirects=max_redirects, **kwargs)
