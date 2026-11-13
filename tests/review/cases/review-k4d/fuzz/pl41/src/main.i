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
with_int = (body) {
	_syn3_0: Svc(Int) = Svc(f = () { 1 }, m = (_: Int) { Nil })
	body() <- (_syn3_0)
}
with_dep = (body) { body() <- (Dep(v = 5)) }
with_cov = (body, v) {
	_syn3_1: Cov(_) = Cov(f = () { v })
	body() <- (_syn3_1)
}
pick = (a, b) { [a, b] }
h0 = () { twice(() { get() }) }
h1 = () {
	_syn3_2: Svc(Int) = Svc(
		f = () {
			call(Cb(f = () { ask() }, m = (_) { Nil })) +
				with_dep(() { with_int(() { get() + ask() }) })
		},
		m = (_: Int) { Nil },
	)
	with_int(() { ask() }) <- (_syn3_2)
}

main = () {
	_syn3_3: Cov(Int) = Cov(f = () { with_cov(() { askc() }, 3) })
	_syn3_4: Cov(Int) = Cov(f = () { twice(() { get() }) })
	_syn3_5: Cov(Int) = Cov(f = () { 2 })
	_syn3_6: Svc(Int) = Svc(f = () { get() }, m = (_: Int) { Nil })
	_syn3_7: Svc(String) = Svc(f = () { "s" }, m = (_: String) { Nil })
	{
		get() + 1 + with_int(() { ask() })
		Nil
	} <- (_syn3_3, _syn3_4, _syn3_5, _syn3_6, _syn3_7)
}
