Dep = (v = Int)
get = () <- (d: Dep) { d.v }
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
Cov(A) = (f = () -> A)
askc = <A>() <- (s: Cov(A)) { s.f() }
Cb(N) = (f = () <- N -> Int, m = (N) -> Nil)
call = <N>(c: Cb(N)) {
	Cb(f = f, ..) = c
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
twice = <N>(body: () <- N -> Int) { body() + body() }
with_int = (body) { body() <- (Svc(f = () { 1 }, m = (_: Int) { Nil })) }
with_dep = (body) { body() <- (Dep(v = 5)) }
with_cov = (body, v) { body() <- (Cov(f = () { v })) }
pick = (a, b) { [a, b] }
h0 = () { 1 }
h1 = () {
	run(() { ask() }) <- (
		Cov(f = () { ask() + 1 + askc() }),
		Cov(
			f = () {
				askc() + call(Cb(f = () { ask() }, m = (_) { Nil })) + ask()
			},
		),
	)
}

main = () {
	{
		h1()
		Nil
	} <- (
		Svc(f = () { get() }, m = (_: Int) { Nil }),
		Cov(f = () { 2 }),
		Dep(v = 1),
	)
}
