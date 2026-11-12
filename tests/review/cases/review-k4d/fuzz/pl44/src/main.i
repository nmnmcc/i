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
h0 = () { run(h0) }
h1 = () {
	with_int(() { ask() }) <- (
		Cov(
			f = () { with_dep(() { with_int(() { get() + ask() }) }) + askc() },
		),
		Svc(f = () { "s" }, m = (_: String) { Nil }),
	)
}

main = () {
	{
		call(Cb(f = () { ask() }, m = (_) { Nil })) +
			twice(h1) +
			with_dep(() { get() })
		Nil
	} <- (
		Cov(f = () { with_int(() { run(() { ask() + 1 }) }) + 1 + askc() }),
		Dep(v = 1),
		Svc(f = () { get() }, m = (_: Int) { Nil }),
	)
}
