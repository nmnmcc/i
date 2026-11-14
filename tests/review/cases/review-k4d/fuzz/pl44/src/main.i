Dep = (v = Int)
get = () <- (d : Dep) { d.v }
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s : Svc(A)) { s.f() }
Cov(A) = (f = () -> A)
askc = <A>() <- (s : Cov(A)) { s.f() }
Cb(N) = (f = () <- N -> Int, m = (N) -> Nil)
call = <N>(c : Cb(N)) {
	Cb(f = f, ..) = c
	f()
}
run = <A, N>(body : () <- N -> A) { body() }
twice = <N>(body : () <- N -> Int) { body() + body() }
with_int = (body) {
	_syn3_0 : Svc(Int) = Svc(f = () { 1 }, m = (_ : Int) { Nil })
	body() <- (_syn3_0)
}
with_dep = (body) { body() <- (Dep(v = 5)) }
with_cov = (body, v) {
	_syn3_1 : Cov(_) = Cov(f = () { v })
	body() <- (_syn3_1)
}
pick = (a, b) { [a, b] }
h0 = () { run(h0) }
h1 = () {
	_syn3_2 : Cov(Int) = Cov(
		f = () { with_dep(() { with_int(() { get() + ask() }) }) + askc() },
	)
	_syn3_3 : Svc(String) = Svc(f = () { "s" }, m = (_ : String) { Nil })
	with_int(() { ask() }) <- (_syn3_2, _syn3_3)
}

main = () {
	_syn3_4 : Cov(Int) = Cov(
		f = () { with_int(() { run(() { ask() + 1 }) }) + 1 + askc() },
	)
	_syn3_5 : Svc(Int) = Svc(f = () { get() }, m = (_ : Int) { Nil })
	{
		call(Cb(f = () { ask() }, m = (_) { Nil })) +
			twice(h1) +
			with_dep(() { get() })
		Nil
	} <- (_syn3_4, Dep(v = 1), _syn3_5)
}
