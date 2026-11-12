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
h0 = () { 1 }

main = () {
	{
		run(() { run(() { usa() + h0() }) + 1 })
		Nil
	} <- (
		S(
			f = () { run(() { h0() + 1 + get() }) },
			m = (_: Dep | S(Dep)) { Nil },
		),
	)
}
