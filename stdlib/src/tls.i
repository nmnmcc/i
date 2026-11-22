//// TLS on connected byte streams.
(
	i/request
	i/hostcall
	i/resource
	i/fail
	i/core
	i/net
)

(
	Tls(..)
	Stream(..)
	TlsError(..)
	options
	identity
	connect
	accept
	dial
	alpn
	read
	write
	shutdown_write
	close
	byte_stream
)

Stream = (Int)

TlsError = (stage = String, reason = String)

@import("i:tls/v1")
Tls = (
	connect = (Int, String, String, BitArray, BitArray, BitArray) -> Result(
		Int,
		String,
	),
	accept = (Int, BitArray, BitArray, String) -> Result(Int, String),
	alpn = (Int) -> Result(String, String),
	read = (Int, Int) -> Result(BitArray, String),
	write = (Int, BitArray) -> Result((), String),
	shutdown_write = (Int) -> Result((), String),
	close = (Int) -> (),
)

options = () -> (
	alpn = List(String),
	roots = BitArray,
	cert = BitArray,
	key = BitArray,
) { (alpn = [], roots = <<>>, cert = <<>>, key = <<>>) }

identity = (cert : BitArray, key : BitArray) -> (
	cert = BitArray,
	key = BitArray,
) { (cert = cert, key = key) }

join = (items : List(String)) -> String {
	items |>
		([]) { "" }
		& ([one]) { one }
		& ([first, ..rest]) { core.concat([first, "\u{0}", join(rest)]) }
}

slice = (text, start, len) {
	core.byte_slice(text, start, len) |>
		(Ok(v)) { v }
		& (Error(_)) { panic("tls: invalid error") }
}

raise = (message) {
	core.index_of(message, ":", 0) |>
		(Ok(n)) {
			fail.raise(
				TlsError(
					stage = slice(message, 0, n),
					reason = slice(
						message,
						n + 1,
						core.byte_size(message) - n - 1,
					),
				),
			)
		}
		& (Error(_)) { panic("tls: invalid error") }
}

finish = (r : hostcall.Call(Result(Int, String))) <- (raw : Tls) {
	number = resource.acquire(
		() {
			resource.ensure(
				() {
					request.await(r) |>
						(Ok(value)) { value }
						& (Error(message)) { raise(message) }
				},
				() {
					request.state(r) |>
						(request.Ready(Ok(value))) { raw.close(value) }
						& (_) { () }
					hostcall.cancel(r)
				},
			)
		},
		(value) { close(Stream(value)) <- (raw) },
	)
	Stream(number)
}

/// Establishes client TLS on `stream` and gives a `Stream` owned by the
/// current `resource.Scope`: its close is attached to the scope before
/// `connect` gives the stream, and runs when the scope ends.
///
/// Needs `resource.Scope`, `Tls` and `Fail(TlsError)`.
connect = (
	stream,
	name,
	opts : (
		alpn = List(String),
		roots = BitArray,
		cert = BitArray,
		key = BitArray,
	)
) <- (raw : Tls) {
	finish(
		hostcall.call(() {
			raw.connect(
				net.stream_number(stream),
				name,
				join(opts.alpn),
				opts.roots,
				opts.cert,
				opts.key,
			)
		}),
	)
}

/// Establishes server TLS on `stream` and gives a `Stream` owned by the
/// current `resource.Scope`: its close is attached to the scope before
/// `accept` gives the stream, and runs when the scope ends.
///
/// Needs `resource.Scope`, `Tls` and `Fail(TlsError)`.
accept = (
	stream,
	ident : (cert = BitArray, key = BitArray),
	opts : (
		alpn = List(String),
		roots = BitArray,
		cert = BitArray,
		key = BitArray,
	)
) <- (raw : Tls) {
	finish(
		hostcall.call(() {
			raw.accept(
				net.stream_number(stream),
				ident.cert,
				ident.key,
				join(opts.alpn),
			)
		}),
	)
}

/// Connects to `host` and `port` and establishes client TLS, giving a
/// `Stream` owned by the current `resource.Scope`. The TCP stream and the
/// TLS stream are attached to the same scope; its close closes both, and a
/// double close does nothing.
///
/// Needs `resource.Scope`, `Tls`, `Net`, `Fail(TlsError)` and
/// `Fail(NetError)`.
dial = (host, port, opts) { connect(net.connect(host, port), host, opts) }

alpn = (Stream(n)) <- (raw : Tls) {
	raw.alpn(n) |> (Ok(value)) { value } & (Error(message)) { raise(message) }
}

read = (Stream(n), max) <- (raw : Tls) {
	raw.read(n, max) |>
		(Ok(value)) { value }
		& (Error(message)) { raise(message) }
}

write = (Stream(n), bytes) <- (raw : Tls) {
	raw.write(n, bytes) |>
		(Ok(value)) { value }
		& (Error(message)) { raise(message) }
}

shutdown_write = (Stream(n)) <- (raw : Tls) {
	raw.shutdown_write(n) |>
		(Ok(value)) { value }
		& (Error(message)) { raise(message) }
}

/// Closes a TLS stream. Closing a TLS stream that is closed already does
/// nothing. The scope that owns the stream calls this; a program does not
/// need to. Closing the TLS stream does not close the underlying TCP stream;
/// `tls.dial` attaches that stream to the same scope, so both close together.
///
/// Needs `Tls`.
close = (Stream(n)) <- (raw : Tls) { raw.close(n) }

byte_stream = (stream) {
	(
		read = (max) { read(stream, max) },
		write = (bytes) { write(stream, bytes) },
		shutdown_write = () { shutdown_write(stream) },
		close = () { close(stream) },
	)
}
