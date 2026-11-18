(
	(Io) = i/io
)
Svc(A) = (f = () -> A, m = (A) -> ())
ask = <A>() <- (s : Svc(A)) { s.f() }
wrap = (body, svc : Svc(String)) {
	_syn3_0 : Svc(String) = svc
	body() <- (_syn3_0)
}
tell = (x : Int) { x |> (2) { "two" } & (_) { "other" } }

main = () <- (io : Io) {
	_syn3_1 : Svc(Int) = Svc(f = () { 1 }, m = (_ : Int) { () })
	io.print(
		tell(
			wrap(
				() { ask() + 1 },
				Svc(f = () { "s" }, m = (_ : String) { () }),
			),
		),
	) <- (_syn3_1)
}
