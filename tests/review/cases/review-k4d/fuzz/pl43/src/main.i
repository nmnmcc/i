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
h0 = () { askc() }
h1 = () { with_dep(() { with_int(() { get() + ask() }) }) }

main = () {
	_syn3_2 : Cov(Int) = Cov(f = () { 2 })
	_syn3_3 : Cov(Int) = Cov(
		f = () { run(() { ask() }) + with_cov(() { askc() }, 3) },
	)
	_syn3_4 : Svc(Int) = Svc(f = () { get() }, m = (_ : Int) { Nil })
	{
		with_dep(() { with_int(() { get() + ask() }) }) +
			with_cov(() { askc() }, 3)
		Nil
	} <- (_syn3_2, _syn3_3, _syn3_4, Dep(v = 1))
}
