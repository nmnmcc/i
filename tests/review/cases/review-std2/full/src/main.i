(i/fail)
Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ^ E ! N, (N) -> Nil) }
make(E, N) = (send: (Entry) -> Nil ^ E ! N) -> Sink(E, N) { Sink(send, (_: N) { Nil }) }
write(E, N) = (entry: Entry, sink ! Sink(E, N)) { Sink(send, _) = sink send(entry) }
example = () {
 ! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
 fail.catch(() { write(Entry(message: "seen")) })
}
