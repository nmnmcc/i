//// TLS on connected byte streams.
(
	i/request
	i/resource
	i/fail
	i/core
	i/net
)

(
	Tls
	Stream
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

Stream {
	Stream(Int)
}

TlsError {
	TlsError(stage: String, reason: String)
}

@host("i:tls/v1", "request")
Tls {
	Tls(
		connect: (
			stream: Int
			name:   String
			alpn:   String
			roots:  BitArray
			cert:   BitArray
			key:    BitArray
		) -> Request(Result(Int, String))
		accept: (
			stream: Int
			cert:   BitArray
			key:    BitArray
			alpn:   String
		) -> Request(Result(Int, String))
		alpn:           (stream: Int) -> Request(Result(String, String))
		read:           (stream: Int, max: Int) -> Request(Result(BitArray, String))
		write:          (stream: Int, bytes: BitArray) -> Request(Result(Nil, String))
		shutdown_write: (stream: Int) -> Request(Result(Nil, String))
		close:          (stream: Int) -> Request(Nil)
	)
}

options = () { (alpn: [], roots: <<>>, cert: <<>>, key: <<>>) }

identity = (cert: BitArray, key: BitArray) { (cert: cert, key: key) }

join = (items: List(String)) -> String {
	items |> {
		[]              -> ""
		[one]           -> one
		[first, ..rest] -> core.concat([first, "\u{0}", join(rest)])
	}
}

slice = (text, start, len) {
	core.byte_slice(text, start, len) |> {
		Ok(v)    -> v
		Error(_) -> panic("tls: invalid error")
	}
}

raise = (message) {
	core.index_of(message, ":", 0) |> {
		Ok(n) -> fail.raise(
			TlsError(
				slice(message, 0, n)
				slice(message, n + 1, core.byte_size(message) - n - 1)
			)
		)
		Error(_) -> panic("tls: invalid error")
	}
}

finish = (r: Request(Result(Int, String)), raw ! Tls) {
	number = resource.ensure(() {
		request.await(r) |> {
			Ok(value)      -> value
			Error(message) -> raise(message)
		}
	}, () {
		request.state(r) |> {
			request.Ready(Ok(value)) -> request.await_owned(raw.close(value))
			_                        -> Nil
		}
		request.cancel(request.ticket(r))
	})
	Stream(number)
}

connect = (stream, name, opts, raw ! Tls) {
	finish(
		raw.connect(
			net.stream_number(stream)
			name
			join(opts.alpn)
			opts.roots
			opts.cert
			opts.key
		)
	)
}

accept = (stream, ident, opts, raw ! Tls) {
	finish(
		raw.accept(
			net.stream_number(stream)
			ident.cert
			ident.key
			join(opts.alpn)
		)
	)
}

dial = (host, port, opts) { connect(net.connect(host, port), host, opts) }

alpn = (Stream(n), raw ! Tls) {
	request.await_owned(raw.alpn(n)) |> {
		Ok(value)      -> value
		Error(message) -> raise(message)
	}
}

read = (Stream(n), max, raw ! Tls) {
	request.await_owned(raw.read(n, max)) |> {
		Ok(value)      -> value
		Error(message) -> raise(message)
	}
}

write = (Stream(n), bytes, raw ! Tls) {
	request.await_owned(raw.write(n, bytes)) |> {
		Ok(value)      -> value
		Error(message) -> raise(message)
	}
}

shutdown_write = (Stream(n), raw ! Tls) {
	request.await_owned(raw.shutdown_write(n)) |> {
		Ok(value)      -> value
		Error(message) -> raise(message)
	}
}

close = (Stream(n), raw ! Tls) { request.await_owned(raw.close(n)) }

byte_stream = (stream) {
	(
		read: (max) { read(stream, max) }
		write: (bytes) { write(stream, bytes) }
		shutdown_write: () { shutdown_write(stream) }
		close: () { close(stream) }
	)
}
