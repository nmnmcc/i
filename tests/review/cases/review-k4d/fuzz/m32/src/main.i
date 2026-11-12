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
	{
		k = () { h1() }
		{
			k() + ut()
			Nil
		} <- (
			S(f = () { get() + 1 }, m = (_: T(Dep)) { Nil }),
			S(
				f = () { run(() { us() }) + get() },
				m = (_: Dep | T(Dep) | S(Dep | T(Dep))) { Nil },
			),
		)
	} <- (
		S(f = () { 1 }, m = (_: Never) { Nil }),
		T(f = () { us() + h0() }, m = (_: S(Never)) { Nil }),
	)
}
