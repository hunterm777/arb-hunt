"""Every test runs with sockets disabled: the package must never open one."""

import socket

import pytest


class _NoNetwork(socket.socket):
    def __init__(self, *args, **kwargs):
        raise AssertionError("arbcalc opened a socket")


@pytest.fixture(autouse=True)
def no_network(monkeypatch):
    monkeypatch.setattr(socket, "socket", _NoNetwork)
    monkeypatch.setattr(socket, "create_connection", _NoNetwork)
    monkeypatch.setattr(socket, "getaddrinfo", lambda *a, **k: (_ for _ in ()).throw(AssertionError("DNS")))
