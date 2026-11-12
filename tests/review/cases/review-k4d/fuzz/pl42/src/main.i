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
h0 = () { twice(() { get() }) }
h1 = () {
	p7 = pick(
		() { get() + ask() },
		() {
			with_dep(() { with_int(() { get() + ask() }) }) +
				call(Cb(f = () { get() }, m = (_: Dep) { Nil }))
		},
	)
	l4 = () { h1() + with_int(() { run(() { ask() + 1 }) }) }
	askc()
}

main = () {
	{
		with_int(() { run(() { ask() + 1 }) }) + askc()
		Nil
	} <- (
		Svc(f = () { twice(() { get() }) }, m = (_: Int) { Nil }),
		Svc(
			f = () { with_dep(() { get() }) + ask() + run(() { ask() }) },
			m = (_: Int) { Nil },
		),
		Cov(f = () { 2 }),
		Svc(f = () { get() }, m = (_: Int) { Nil }),
		Dep(v = 1),
	)
}
