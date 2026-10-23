# SPDX-License-Identifier: MPL-2.0
"""Small deterministic host probes, without contacting remote services."""

import errno
import socket


NETWORK_UNAVAILABLE = {
    errno.EPERM,
    errno.EACCES,
    errno.EAFNOSUPPORT,
    errno.EPROTONOSUPPORT,
    errno.ENOSYS,
}


def network_permission_error(error):
    return error.errno in NETWORK_UNAVAILABLE


def local_network_unavailable():
    try:
        for kind in (socket.SOCK_STREAM, socket.SOCK_DGRAM):
            with socket.socket(socket.AF_INET, kind) as connection:
                connection.bind(("127.0.0.1", 0))
    except OSError as error:
        if network_permission_error(error):
            return f"local sockets unavailable: {error}"
        raise
    return None
