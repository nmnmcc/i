(
	(Io) = i/io
)
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
tell = (x: Int) { x |> (2) { "two" } | (_) { "other" } }

main = () <- (io: Io) {
	_syn3_0: Svc(Int) = Svc(f = () { 1 }, m = (_: Int) { Nil })
	{
		body = () { ask() + 1 }
		inner: <N>(() <- N -> Int) <- (Fail(Never), ..N) -> Int = (b) <- (
			fail: Fail(Never),
		) -> Int {
			_syn3_1: Svc(String) = Svc(f = () { "s" }, m = (_: String) { Nil })
			b() <- (_syn3_1)
		}
		io.print(
			tell(inner(body)),
		)
	} <- (_syn3_0)
}
