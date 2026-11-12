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
	{
		a = h1()
		{
			a + ut() + run(() { ut() })
			Nil
		} <- (Dep(v = 3), T(f = () { h0() }, m = (_: Dep | S(Dep)) { Nil }))
	} <- (S(f = () { get() }, m = (_: Dep) { Nil }))
}
