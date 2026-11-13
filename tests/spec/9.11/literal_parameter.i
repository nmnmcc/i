// spec: 9.11, 10.3
// expect: pass
// output: echo a
// Spec 10.3 after T1 permits finite outgoing String sets; the host answer stays
// a full String. The testing host echoes its input through the same request cell.
(
	i/request
)

@host("test:async/v1", "request")
Later {
	Later(echo: ("a" | "b") -> Request(String))
}


main = (later ! Later) { assert(request.await_owned(later.echo("a")) == "a") }
