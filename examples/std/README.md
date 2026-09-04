# Wave standard library examples

Each `.wave` file is a small, independent program focused on one standard-library area.
Run one from the repository root with:

```sh
wavec run examples/std/strings.wave
```

The filesystem example creates and removes a file in the current directory. The environment,
I/O, process, and time examples use the host operating-system provider selected by `wavec`.

The portable cookbook examples include `checked_math.wave` for result-based arithmetic,
`binary_packet.wave` for endian-aware cursor I/O, `text_paths.wave` for path and string
composition, `datetime_roundtrip.wave` for calendar conversion and formatting, and
`buffer_builder.wave` for owned growable byte buffers.

`network.wave` covers IPv4/IPv6 parsing, formatting, and address conversion.
`net_tcp.wave` enforces TCP loopback, timeouts, socket options, and EOF;
`net_udp.wave` covers UDP peer addresses and zero-byte datagrams; and
`net_ipv6.wave` runs TCP and UDP over IPv6 loopback. `net_dns.wave` resolves
`localhost` through the hosted resolver without depending on external network access.
`net_unix.wave` verifies Unix-domain stream ownership and path cleanup on Unix,
and verifies the explicit unsupported result on Windows.
`net_interfaces.wave` enumerates hosted interface addresses into caller-owned storage.
`net_vectored.wave` verifies one scatter/gather send and receive across a TCP stream.
`net_event.wave` verifies portable, level-triggered readiness with an application token;
the selected backend is epoll, kqueue, or the bounded Windows WSAPoll provider.
