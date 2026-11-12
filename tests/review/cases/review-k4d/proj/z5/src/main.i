(
	(Io) = i/io
)
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
wrap = <N>(k: (() <- N -> Int) <- N -> Int, svc: Svc(String)) -> Int {
	first = () -> Int { k(() { 1 }) <- (svc) }
	first() + k(() { ask() + 1 })
}
tell = (x: Int) { x |> (5) { "five" } | (_) { "other" } }

main = () <- (io: Io) {
	io.print(
		tell(
			wrap(
				(b) { b() + ask() },
				Svc(f = () { "s" }, m = (_: String) { Nil }),
			),
		),
	) <- (Svc(f = () { 1 }, m = (_: Int) { Nil }))
}
