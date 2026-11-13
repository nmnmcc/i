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
h0 = () { us() }
h1 = () { run(h0) + run(() { ut() }) }

main = () {
	_syn3_0: S(Never) = S(f = () { 1 }, m = (_: Never) { Nil })
	_syn3_1: T(S(Never)) = T(f = () { us() + h0() }, m = (_: S(Never)) { Nil })
	{
		k = () { h1() }
		k() + ut()
		Nil
	} <- (_syn3_0, _syn3_1)
}
