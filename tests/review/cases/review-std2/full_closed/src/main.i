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
send : (Entry) <- (Fail(String), Sink(String, Never)) -> () = write

example = () {
	{
		Error("seen") = fail.catch(() { send(Entry(message = "seen")) })
		()
	} <- (make((entry) { fail.raise(entry.message) }))
}
