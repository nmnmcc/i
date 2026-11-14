(
	i/fail
)
Entry = (message = String)
Sink(E, N) = ((Entry) <- (Fail(E), ..N) -> Nil, (N) -> Nil)
make = <E, N>(send : (Entry) <- (Fail(E), ..N) -> Nil) -> Sink(E, N) {
	Sink(send, (_ : N) { Nil })
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
