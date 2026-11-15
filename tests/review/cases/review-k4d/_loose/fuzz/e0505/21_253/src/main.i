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
h0 = () { run(() { us() }) + ut() }
h1 = () { usa() }

main = () {
	_syn3_0 : S(S(T(Dep))) = S(
		f = () { h1() + h1() },
		m = (_ : S(T(Dep))) { Nil },
	)
	_syn3_1 : T(Dep) = T(f = () { get() }, m = (_ : Dep) { Nil })
	{
		w = () { get() + run(() { us() }) + get() <- (Dep(v = 1)) }
		_syn3_2 : S(Dep | T(Dep)) = S(
			f = () { ut() + get() },
			m = (_ : { Dep | T(Dep) }) { Nil },
		)
		{
			l = () { run(() { us() + usa() }) }
			h0() + l() + ut() + h1() + w()
			Nil
		} <- (_syn3_2)
	} <- (_syn3_0, _syn3_1)
}
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `Dep`, which must be given before it gets to the entry point
// src/main.i:22:42: error[E0501]: `T(Dep)` is not a subtype of `S(T(Dep))` (the type at 22:20 needs it)
// src/main.i:24:3: error[E0505]: this type has itself inside it, through a type variable (§4.7)
// 3 errors
//
