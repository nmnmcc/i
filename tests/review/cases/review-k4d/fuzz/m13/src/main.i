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
h0 = () { run(() { us() }) + ut() }
h1 = () { usa() }

main = () {
	{
		l = () { run(() { us() + usa() }) }
		h0() + l() + ut() + h1()
		Nil
	} <- (
		Dep(v = 1),
		T(f = () { get() }, m = (_: Dep) { Nil }),
		S(f = () { ut() + get() }, m = (_: Dep | T(Dep)) { Nil }),
	)
}
