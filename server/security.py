import ipaddress
import socket
import urllib.parse


def normalize_public_base_url(url: str) -> str:
    base_url = (url or "").strip().rstrip("/")
    if not base_url:
        raise ValueError("中转地址不能为空")
    if not base_url.startswith(("http://", "https://")):
        base_url = "https://" + base_url

    parsed = urllib.parse.urlparse(base_url)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise ValueError("请求地址不合法")

    try:
        addr_info = socket.getaddrinfo(parsed.hostname, None)
    except OSError as exc:
        raise ValueError("请求地址无法解析") from exc

    for _, _, _, _, sockaddr in addr_info:
        ip_str = sockaddr[0].split("%", 1)[0]
        ip = ipaddress.ip_address(ip_str)
        if (
            ip.is_private
            or ip.is_loopback
            or ip.is_link_local
            or ip.is_reserved
            or ip.is_unspecified
            or ip.is_multicast
        ):
            raise ValueError("请求地址不合法 (IP 为私有、回环或保留地址)")

    return base_url
