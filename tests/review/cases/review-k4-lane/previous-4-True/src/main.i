(
	i/fail
)

Entry {
	Entry(message: String)
}

Sink(E, N) {
	Sink((Entry) -> Nil ^ E ! N, (N) -> Nil)
}

write(E, N) = (entry: Entry, sink ! Sink(E, N)) {
	Sink(send, _) = sink
	send(entry)
}

example = () { fail.catch(() { write(Entry("seen")) }) }


main = () { example() }
