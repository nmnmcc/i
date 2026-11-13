# TLS on the Rust host

The Rust standard host provides TLS.

`i/tls` upgrades a connected `i/net` stream. `connect(stream, server_name,
options)` performs a client handshake, validates the peer certificate and
returns an encrypted byte stream. `accept(stream, identity, options)` performs
a server handshake using the identity's PEM certificate chain and private key.
`dial(host, port, options)` connects by name and uses that host as the server
name. An upgrade consumes the plain stream; it must acquire both directions and
fails with `busy` if either has an operation in progress.

`options()` returns `(alpn : [], roots : <<>>, cert : <<>>, key : <<>>)`.
Set `alpn` to an ordered list of protocol strings, `roots` to extra PEM trust
roots, and `cert` and `key` to an optional PEM client identity.
`identity(cert, key)` constructs a server identity. The system roots are included by default. The
server name is used for certificate verification and SNI. Server options supply
the ALPN list. The negotiated protocol is available on the encrypted stream;
if no protocol was negotiated `alpn(stream)` returns the empty string. PEM inputs are bytes.

A byte stream exposes read with a positive bound, write all bytes,
`shutdown_write`, and close as ordinary i functions. Both TCP and TLS provide
that same shape through `net.byte_stream(stream)` and `tls.byte_stream(stream)`:
records with `read`, `write`, `shutdown_write`, and `close` functions. Reads can return fewer bytes than requested. EOF is an empty
byte array. One read and one write may run together; a second operation in the
same direction fails with `busy`. Close is idempotent and settles pending
operations with `closed` unless they already completed. TLS `shutdown_write`
sends close_notify and leaves the read side open.

Failures are `TlsError(stage, reason)`, where stage is `handshake`,
`certificate`, or `io` and reason describes the failure. Certificate and server
name validation failures have the `certificate` stage. The host implements
TLS using rustls with system roots; the JavaScript and WASI hosts are unchanged.
Session resumption tuning, UDP/DTLS, and QUIC are outside this API.
