Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
T(N) = (f = () <- N -> Int, m = (N) -> Nil)
us = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
ut = <N>() <- (t: T(N)) {
	T(f = f, ..) = t
	f()
}
usa = <N>() <- (s: S(N)) -> Int {
	S(f = f, ..) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
h0 = () { get() + us() }
h1 = () { h0() <- (Dep(v = 2)) }

main = () {
	w = () {
		_syn3_0: S(Dep) = S(f = () { get() }, m = (_: Dep) { Nil })
		run(() { ut() + get() }) <- (_syn3_0)
	}
	a = h1()
	_syn3_1: T(Dep | S(Dep)) = T(f = () { h0() }, m = (_: Dep | S(Dep)) { Nil })
	{
		a + ut() + run(() { ut() }) + w()
		Nil
	} <- (_syn3_1, Dep(v = 3))
}
// src/main.i:24:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `S(Dep)`, which must be given before it gets to the entry point
// src/main.i:24:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `S(a)`, which must be given before it gets to the entry point
// src/main.i:24:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `a`, which must be given before it gets to the entry point
// src/main.i:25:3: error[E0505]: this type has itself inside it, through a type variable (§4.7)
// 4 errors
//
