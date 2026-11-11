//// Byte-oriented standard streams.
(
	i/request
	i/fail
)

(
	Stdio(..)
	StdioError(..)
	read
	write
	write_error
	flush
	flush_error
	print
	print_error
)

StdioError {
	StdioError(operation: String, code: String)
}

@host("i:stdio/v1", "request")
Stdio {
	Stdio(
		read:        (max_bytes: Int) -> Request(Result(BitArray, String))
		write:       (bytes: BitArray) -> Request(Result(Nil, String))
		write_error: (bytes: BitArray) -> Request(Result(Nil, String))
		flush:       () -> Request(Result(Nil, String))
		flush_error: () -> Request(Result(Nil, String))
	)
}

read = (max_bytes: Int, io ! Stdio) {
	request.await_owned(io.read(max_bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(StdioError("read", code))
	}
}

write = (bytes: BitArray, io ! Stdio) {
	request.await_owned(io.write(bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(StdioError("write", code))
	}
}

write_error = (bytes: BitArray, io ! Stdio) {
	request.await_owned(io.write_error(bytes)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(StdioError("write_error", code))
	}
}

flush = (io ! Stdio) {
	request.await_owned(io.flush()) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(StdioError("flush", code))
	}
}

flush_error = (io ! Stdio) {
	request.await_owned(io.flush_error()) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(StdioError("flush_error", code))
	}
}

print = (text: String) {
	line = text <> "\n"
	write(<<line:utf8>>)
	flush()
}

print_error = (text: String) {
	line = text <> "\n"
	write_error(<<line:utf8>>)
	flush_error()
}
