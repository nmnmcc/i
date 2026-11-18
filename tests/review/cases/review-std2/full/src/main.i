(
	i/fail
)
Entry = (message = String)
Sink(E, N) = ((Entry) <- (Fail(E), ..N) -> (), (N) -> ())
make = <E, N>(send : (Entry) <- (Fail(E), ..N) -> ()) -> Sink(E, N) {
	Sink(send, (_ : N) { () })
}
write = <E, N>(entry : Entry) <- (sink : Sink(E, N)) {
	Sink(send, _) = sink
	send(entry)
}
example = () {
	fail.catch(() { write(Entry(message = "seen")) }) <- (
		make((entry) { fail.raise(entry.message) }),
	)
}
