(i/fail)
Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ^ E ! N, (N) -> Nil) }
make(E, N) = (send: (Entry) -> Nil ^ E ! N) -> Sink(E, N) { Sink(send, (_: N) { Nil }) }
write(E, N) = (entry: Entry, sink ! Sink(E, N)) { Sink(send, _) = sink send(entry) }
send: (Entry) -> Nil ^ String ! Sink(String,Never) = write

main = () {
 ! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
 Error("seen") = fail.catch(() { send(Entry("seen")) })
 Nil
}
