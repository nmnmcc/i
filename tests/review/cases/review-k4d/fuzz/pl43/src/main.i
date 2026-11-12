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
h0 = () { askc() }
h1 = () { with_dep(() { with_int(() { get() + ask() }) }) }

main = () {
	{
		with_dep(() { with_int(() { get() + ask() }) }) +
			with_cov(() { askc() }, 3)
		Nil
	} <- (
		Cov(f = () { 2 }),
		Cov(f = () { run(() { ask() }) + with_cov(() { askc() }, 3) }),
		Svc(f = () { get() }, m = (_: Int) { Nil }),
		Dep(v = 1),
	)
}
