(
	(Io) = i/io
)
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
wrap = (body, svc: Svc(String)) { body() <- (svc) }
tell = (x: Int) { x |> (2) { "two" } | (_) { "other" } }

main = () <- (io: Io) {
	io.print(
		tell(
			wrap(
				() { ask() + 1 },
				Svc(f = () { "s" }, m = (_: String) { Nil }),
			),
		),
	) <- (Svc(f = () { 1 }, m = (_: Int) { Nil }))
}
