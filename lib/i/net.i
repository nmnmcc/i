//// TCP networking and name resolution.
(
	i/request
	i/resource
	i/fail
	i/core
)

(
	Net
	Listener
	Stream
	NetError(..)
	listen
	local_port
	accept
	connect
	resolve
	read
	write
	shutdown_write
	close_stream
	close_listener
	stream_number
	byte_stream
)

Listener {
	Listener(Int)
}

Stream {
	Stream(Int)
}

NetError {
	NetError(operation: String, endpoint: String, code: String)
}

@host("i:net/v1", "request")
Net {
	Net(
		listen:         (address: String, port: Int) -> Request(Result(Int, String))
		local_port:     (listener: Int) -> Request(Result(Int, String))
		accept:         (listener: Int) -> Request(Result(Int, String))
		connect:        (address: String, port: Int) -> Request(Result(Int, String))
		resolve:        (name: String) -> Request(Result(String, String))
		read:           (stream: Int, max_bytes: Int) -> Request(Result(BitArray, String))
		write:          (stream: Int, bytes: BitArray) -> Request(Result(Nil, String))
		shutdown_write: (stream: Int) -> Request(Result(Nil, String))
		close_stream:   (stream: Int) -> Request(Nil)
		close_listener: (listener: Int) -> Request(Nil)
	)
}

listen = (address, port, raw ! Net) {
	r = raw.listen(address, port)
	number = resource.ensure(() {
		request.await(r) |> {
			Ok(value)   -> value
			Error(code) -> fail.raise(NetError("listen", address, code))
		}
	}, () {
		request.state(r) |> {
			request.Ready(Ok(value)) -> request.await_owned(
				raw.close_listener(value)
			)
			_ -> Nil
		}
		request.cancel(request.ticket(r))
	})
	Listener(number)
}

connect = (address, port, raw ! Net) {
	r = raw.connect(address, port)
	number = resource.ensure(() {
		request.await(r) |> {
			Ok(value)   -> value
			Error(code) -> fail.raise(NetError("connect", address, code))
		}
	}, () {
		request.state(r) |> {
			request.Ready(Ok(value)) -> request.await_owned(
				raw.close_stream(value)
			)
			_ -> Nil
		}
		request.cancel(request.ticket(r))
	})
	Stream(number)
}

accept = (Listener(number), raw ! Net) {
	r = raw.accept(number)
	number = resource.ensure(() {
		request.await(r) |> {
			Ok(value)   -> value
			Error(code) -> fail.raise(NetError("accept", "", code))
		}
	}, () {
		request.state(r) |> {
			request.Ready(Ok(value)) -> request.await_owned(
				raw.close_stream(value)
			)
			_ -> Nil
		}
		request.cancel(request.ticket(r))
	})
	Stream(number)
}

local_port = (Listener(number), raw ! Net) {
	request.await_owned(raw.local_port(number)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(NetError("local_port", "", code))
	}
}

read = (Stream(number), max_bytes, raw ! Net) {
	request.await_owned(raw.read(number, max_bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(NetError("read", "", code))
	}
}

write = (Stream(number), bytes, raw ! Net) {
	request.await_owned(raw.write(number, bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(NetError("write", "", code))
	}
}

shutdown_write = (Stream(number), raw ! Net) {
	request.await_owned(raw.shutdown_write(number)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(NetError("shutdown_write", "", code))
	}
}

close_stream = (Stream(number), raw ! Net) {
	request.await_owned(raw.close_stream(number))
}

close_listener = (Listener(number), raw ! Net) {
	request.await_owned(raw.close_listener(number))
}

slice = (text, start, length) {
	core.byte_slice(text, start, length) |> {
		Ok(value) -> value
		Error(_)  -> panic("net: invalid string slice")
	}
}

parts = (text: String, start: Int) -> List(String) {
	core.index_of(text, "\u{0}", start) |> {
		Ok(end)  -> [slice(text, start, end - start), ..parts(text, end + 1)]
		Error(_) -> []
	}
}

resolve = (name, raw ! Net) {
	request.await_owned(raw.resolve(name)) |> {
		Ok(value)   -> parts(value, 0)
		Error(code) -> fail.raise(NetError("resolve", name, code))
	}
}

/// The host handle used when upgrading a TCP stream.
stream_number = (Stream(number)) { number }

/// A transport-independent set of byte stream operations.
byte_stream = (stream) {
	(
		read: (max) { read(stream, max) }
		write: (bytes) { write(stream, bytes) }
		shutdown_write: () { shutdown_write(stream) }
		close: () { close_stream(stream) }
	)
}
