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

main = () {
	l0_0 = () { ut() }
	_syn3_0: T(T(S(Never))) = T(
		f = () { get() + ut() + us() },
		m = (_: T(S(Never))) { Nil },
	)
	{
		ut()
		Nil
	} <- (_syn3_0)
}
