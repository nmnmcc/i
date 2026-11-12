(
	testlib = (test) = std/test
	i/stdio
	i/request
	i/fail
)

test("stdio_fake", () {
	! stdio.Stdio = stdio.Stdio(read: (n) {
		request.ready({ n > 0 } |> {
			True  -> Ok(<<>>)
			False -> Error("invalid")
		})
	}, write: (bytes) {
		request.ready({ bytes == <<"hello\n":utf8>> } |> {
			True  -> Ok(Nil)
			False -> Error("wrong_bytes")
		})
	}, write_error: (bytes) {
		request.ready({ bytes == <<"error\n":utf8>> } |> {
			True  -> Ok(Nil)
			False -> Error("wrong_bytes")
		})
	}, flush: () { request.ready(Ok(Nil)) }, flush_error: () {
		request.ready(Ok(Nil))
	})
	assert(stdio.read(1) == <<>>)
	assert(
		fail.catch(() { stdio.read(0) }) ==
			Error(stdio.StdioError(operation: "read", code: "invalid"))
	)
	stdio.print("hello")
	stdio.print_error("error")
	Nil
})
