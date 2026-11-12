(
	i/fail
)

Entry = (message = String)

Sink(E, N) = ((Entry) <- (Fail(E), ..N) -> Nil, (N) -> Nil)

write = <E, N>(entry: Entry) <- (sink: Sink(E, N)) -> Nil {
	Sink(send, _) = sink
	send(entry)
}

example = () { fail.catch(() { write(Entry(message = "seen")) }) }

main = () { example() }
