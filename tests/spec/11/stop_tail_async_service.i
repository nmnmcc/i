// spec: 11.7, B.2, B.5
// expect: stop "host: value is not correct"
// at: spec/main:26:2
// at: spec/main:37:2
// output: echo one
// output: echo two
// output: two
// output: echo before
// output: before

// A tail call of a function of an async service keeps its position as the
// first one while the program waits for the host, and the value that the
// host gives is checked with it.
@host("test:async/v1", "async")
Later = (nan = () -> Float, echo = (String) -> String)

@host("test:io/v1")
Io = (print = (String) -> Nil)

take = () <- (later: Later, io: Io) {
	io.print(later.echo("before"))
	later.nan()
}

twice = () <- (later: Later) {
	later.echo("one")
	later.echo("two")
}

main = () <- (io: Io) {
	io.print(twice())
	take()
	Nil
}
