(
	i/stdio
	i/request
	i/fail
)

@test
stdio_fake = () {
	! stdio.Stdio = stdio.Stdio((n) {
		request.ready({ n > 0 } |> {
			True  -> Ok(<<>>)
			False -> Error("invalid")
		})
	}, (bytes) {
		request.ready({ bytes == <<"hello\n":utf8>> } |> {
			True  -> Ok(Nil)
			False -> Error("wrong_bytes")
		})
	}, (bytes) {
		request.ready({ bytes == <<"error\n":utf8>> } |> {
			True  -> Ok(Nil)
			False -> Error("wrong_bytes")
		})
	}, () { request.ready(Ok(Nil)) }, () { request.ready(Ok(Nil)) })
	assert(stdio.read(1) == <<>>)
	assert(
		fail.catch(() { stdio.read(0) }) ==
			Error(stdio.StdioError("read", "invalid"))
	)
	stdio.print("hello")
	stdio.print_error("error")
	Nil
}
