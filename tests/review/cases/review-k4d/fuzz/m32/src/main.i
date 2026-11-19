Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> ())
T(N) = (f = () <- N -> Int, m = (N) -> ())
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
	_syn3_0 : S(Never) = S(f = () { 1 }, m = (_ : Never) { () })
	_syn3_1 : T(S(Never)) = T(
		f = () { us() + h0() },
		m = (_ : S(Never)) { () },
	)
	{
		k = () { h1() }
		_syn3_2 : S(T(Dep)) = S(f = () { get() + 1 }, m = (_ : T(Dep)) { () })
		_syn3_3 : S(Dep | T(Dep) | S(Dep | T(Dep))) = S(
			f = () { run(() { us() }) + get() },
			m = (_ : { Dep | T(Dep) | S(Dep | T(Dep)) }) { () },
		)
		{
			k() + ut()
			()
		} <- (_syn3_2, _syn3_3)
	} <- (_syn3_0, _syn3_1)
}
