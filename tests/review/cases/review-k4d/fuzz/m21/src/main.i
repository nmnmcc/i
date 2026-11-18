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
h0 = () { run(() { us() }) + ut() }
h1 = () { usa() }

main = () {
	_syn3_0 : S(S(Never)) = S(
		f = () { run(() { ut() + get() }) },
		m = (_ : S(Never)) { () },
	)
	_syn3_1 : T(Dep) = T(f = () { get() }, m = (_ : Dep) { () })
	{
		l = () { run(() { us() + usa() }) }
		h0() + l() + ut() + h1()
		()
	} <- (_syn3_0, Dep(v = 1), _syn3_1)
}
