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
h0 = () {
	ut() <- (
		S(f = () { 1 + us() + usa() }, m = (_: T(T(T(Never)))) { Nil }),
		T(f = () { run(() { 2 + 2 }) }, m = (_: Never) { Nil }),
	)
}
h1 = () {
	l0_0 = () { get() <- (S(f = () { get() + ut() }, m = (_: Dep) { Nil })) }
	l0_0() + get() <- (
		S(f = () { get() }, m = (_: Never) { Nil }),
		T(
			f = () { 1 + run(() { h0() + 1 }) + ut() },
			m = (_: S(Never)) { Nil },
		),
	)
}
h2 = () {
	l0_0 = () {
		l1_0 = () {
			h0() + ut() <- (
				T(f = () { ut() + us() }, m = (_: S(Dep)) { Nil }),
				S(f = () { usa() + usa() + 1 }, m = (_: Dep) { Nil }),
			)
		}
		usa() + h1() <- (
			S(f = () { l1_0() + 1 + us() }, m = (_: S(Never)) { Nil }),
		)
	}
	run(() { get() + l0_0() + us() }) + run(() { us() + 1 }) <- (
		Dep(v = 1),
		T(f = () { run(() { 2 + 2 + get() }) }, m = (_: Never) { Nil }),
	)
}

main = () {
	ut()
	Nil
}
