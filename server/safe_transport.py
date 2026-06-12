import socket
import sys

from requests.adapters import HTTPAdapter
from urllib3.connection import HTTPConnection, HTTPSConnection
from urllib3.connectionpool import HTTPConnectionPool, HTTPSConnectionPool
from urllib3.exceptions import ConnectTimeoutError, NameResolutionError, NewConnectionError
from urllib3.poolmanager import PoolManager
from urllib3.util.connection import _DEFAULT_TIMEOUT, _set_socket_options

from security import iter_safe_addresses


def _connect_pinned_address(connection: HTTPConnection, allow_private: bool) -> socket.socket:
    err = None
    try:
        safe_addresses = list(
            iter_safe_addresses(connection._dns_host, connection.port, allow_private=allow_private)
        )
    except ValueError as exc:
        raise NameResolutionError(connection.host, connection, exc) from exc

    if not safe_addresses:
        raise NameResolutionError(
            connection.host,
            connection,
            ValueError("请求地址不合法 (无可用公网 IP)"),
        )

    for family, socktype, proto, sockaddr, _ip_str in safe_addresses:
        sock = None
        try:
            sock = socket.socket(family, socktype, proto)
            _set_socket_options(sock, connection.socket_options)
            if connection.timeout is not _DEFAULT_TIMEOUT:
                sock.settimeout(connection.timeout)
            if connection.source_address:
                sock.bind(connection.source_address)
            sock.connect(sockaddr)
            err = None
            return sock
        except OSError as exc:
            err = exc
            if sock is not None:
                sock.close()

    if err is not None:
        raise err
    raise OSError("getaddrinfo returns no safe address")


def _make_connection_cls(base, allow_private: bool):
    class _PinnedConnection(base):
        def _new_conn(self) -> socket.socket:
            try:
                sock = _connect_pinned_address(self, allow_private)
            except socket.gaierror as exc:
                raise NameResolutionError(self.host, self, exc) from exc
            except TimeoutError as exc:
                raise ConnectTimeoutError(
                    self,
                    f"Connection to {self.host} timed out. (connect timeout={self.timeout})",
                ) from exc
            except OSError as exc:
                raise NewConnectionError(
                    self,
                    f"Failed to establish a new connection: {exc}",
                ) from exc

            sys.audit("http.client.connect", self, self.host, self.port)
            return sock

    return _PinnedConnection


def build_safe_pool_manager(allow_private: bool, **kwargs) -> PoolManager:
    http_connection = _make_connection_cls(HTTPConnection, allow_private)
    https_connection = _make_connection_cls(HTTPSConnection, allow_private)

    class _HttpPool(HTTPConnectionPool):
        ConnectionCls = http_connection

    class _HttpsPool(HTTPSConnectionPool):
        ConnectionCls = https_connection

    pool_manager = PoolManager(**kwargs)
    pool_manager.pool_classes_by_scheme = {
        "http": _HttpPool,
        "https": _HttpsPool,
    }
    return pool_manager


class SSRFSafeAdapter(HTTPAdapter):
    def __init__(self, *args, allow_private: bool = False, **kwargs):
        self._allow_private = allow_private
        super().__init__(*args, **kwargs)

    def init_poolmanager(self, connections, maxsize, block=False, **kwargs):
        self.poolmanager = build_safe_pool_manager(
            self._allow_private,
            num_pools=connections,
            maxsize=maxsize,
            block=block,
            **kwargs,
        )

    def proxy_manager_for(self, *args, **kwargs):
        raise RuntimeError("SSRF-safe sessions must be used without an outbound proxy.")
