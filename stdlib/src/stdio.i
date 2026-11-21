//// Byte-oriented standard streams.
(
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

StdioError = (operation = String, code = String)

@import("i:stdio/v1")
Stdio = (
	read = (Int) -> Result(BitArray, String),
	write = (BitArray) -> Result((), String),
	write_error = (BitArray) -> Result((), String),
	flush = () -> Result((), String),
	flush_error = () -> Result((), String),
)

read = (max_bytes : Int) <- (io : Stdio) {
	io.read(max_bytes) |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(StdioError(operation = "read", code = code))
		}
}

write = (bytes : BitArray) <- (io : Stdio) {
	io.write(bytes) |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(StdioError(operation = "write", code = code))
		}
}

write_error = (bytes : BitArray) <- (io : Stdio) {
	io.write_error(bytes) |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(StdioError(operation = "write_error", code = code))
		}
}

flush = () <- (io : Stdio) {
	io.flush() |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(StdioError(operation = "flush", code = code))
		}
}

flush_error = () <- (io : Stdio) {
	io.flush_error() |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(StdioError(operation = "flush_error", code = code))
		}
}

print = (text : String) {
	line = text <> "\n"
	write(<<line:utf8>>)
	flush()
}

print_error = (text : String) {
	line = text <> "\n"
	write_error(<<line:utf8>>)
	flush_error()
}
