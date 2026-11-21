(
	testlib = (test) = i/test
	i/stdio
	i/request
	i/fail
)

test("stdio_fake", () {
	{
		assert(stdio.read(1) == <<>>)
		assert(
			fail.catch(() { stdio.read(0) }) ==
				Error(stdio.StdioError(operation = "read", code = "invalid")),
		)
		stdio.print("hello")
		stdio.print_error("error")
		()
	} <- (
		stdio.Stdio(
			read = (n) {
				{ n > 0 } |>
					(True()) { Ok(<<>>) }
					& (False()) { Error("invalid") }
			},
			write = (bytes) {
				{ bytes == <<"hello\n":utf8>> } |>
					(True()) { Ok(()) }
					& (False()) { Error("wrong_bytes") }
			},
			write_error = (bytes) {
				{ bytes == <<"error\n":utf8>> } |>
					(True()) { Ok(()) }
					& (False()) { Error("wrong_bytes") }
			},
			flush = () { Ok(()) },
			flush_error = () { Ok(()) },
		),
	)
})
