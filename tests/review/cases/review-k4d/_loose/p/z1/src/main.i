(
	(Io) = i/io
)
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
wrap: <N>(() <- N -> Int, Svc(String)) <- (Fail(Never), ..N) -> Int = (
	body,
	svc
) <- (fail: Fail(Never)) -> Int {
	_syn3_0: Svc(String) = svc
	body() <- (_syn3_0)
}
tell = (x: Int) { x |> (2) { "two" } | (_) { "other" } }

main = () <- (io: Io) {
	_syn3_1: Svc(Int) = Svc(f = () { 1 }, m = (_: Int) { Nil })
	io.print(
		tell(
			wrap(
				() { ask() + 1 },
				Svc(f = () { "s" }, m = (_: String) { Nil }),
			),
		),
	) <- (_syn3_1)
}
