Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
T(N) = (f = () <- N -> Int, m = (N) -> Nil)
us = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}
ut = <N>() <- (t : T(N)) {
	T(f = f, ..) = t
	f()
}
usa = <N>() <- (s : S(N)) -> Int {
	S(f = f, ..) = s
	f()
}
run = <A, N>(body : () <- N -> A) { body() }
h0 = () { us() }
h1 = () { run(h0) + run(() { ut() }) }

main = () {
	k = () { h1() }
	w = () {
		_syn3_0 : T(S(Never)) = T(
			f = () { us() + h0() },
			m = (_ : S(Never)) { Nil },
		)
		usa() + get() + run(() { us() }) <- (_syn3_0)
	}
	k() + usa() + w()
	Nil
}
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `Dep`, which must be given before it gets to the entry point
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `S(a)`, which must be given before it gets to the entry point
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `T(a)`, which must be given before it gets to the entry point
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `a`, which must be given before it gets to the entry point
// src/main.i:23:3: error[E0505]: this type has itself inside it, through a type variable (§4.7)
// 5 errors
//
