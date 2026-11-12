(
	(Io) = i/io
)
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
tell = (x: Int) { x |> (2) { "two" } | (_) { "other" } }

main = () <- (io: Io) {
	{
		body = () { ask() + 1 }
		inner = <N>(b: () <- N -> Int) -> Int {
			b() <- (Svc(f = () { "s" }, m = (_: String) { Nil }))
		}
		io.print(
			tell(inner(body)),
		)
	} <- (Svc(f = () { 1 }, m = (_: Int) { Nil }))
}
