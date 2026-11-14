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
h0 = () {
	_syn3_0 : S(T(T(T(Never)))) = S(
		f = () { 1 + us() + usa() },
		m = (_ : T(T(T(Never)))) { Nil },
	)
	_syn3_1 : T(Never) = T(
		f = () { run(() { 2 + 2 }) },
		m = (_ : Never) { Nil },
	)
	ut() <- (_syn3_0, _syn3_1)
}
h1 = () {
	l0_0 = () {
		_syn3_2 : S(Dep) = S(f = () { get() + ut() }, m = (_ : Dep) { Nil })
		get() <- (_syn3_2)
	}
	_syn3_3 : S(Never) = S(f = () { get() }, m = (_ : Never) { Nil })
	_syn3_4 : T(S(Never)) = T(
		f = () { 1 + run(() { h0() + 1 }) + ut() },
		m = (_ : S(Never)) { Nil },
	)
	l0_0() + get() <- (_syn3_3, _syn3_4)
}
h2 = () {
	l0_0 = () {
		l1_0 = () {
			_syn3_5 : T(S(Dep)) = T(
				f = () { ut() + us() },
				m = (_ : S(Dep)) { Nil },
			)
			_syn3_6 : S(Dep) = S(
				f = () { usa() + usa() + 1 },
				m = (_ : Dep) { Nil },
			)
			h0() + ut() <- (_syn3_5, _syn3_6)
		}
		_syn3_7 : S(S(Never)) = S(
			f = () { l1_0() + 1 + us() },
			m = (_ : S(Never)) { Nil },
		)
		usa() + h1() <- (_syn3_7)
	}
	_syn3_8 : T(Never) = T(
		f = () { run(() { 2 + 2 + get() }) },
		m = (_ : Never) { Nil },
	)
	run(() { get() + l0_0() + us() }) + run(() { us() + 1 }) <- (
		Dep(v = 1),
		_syn3_8,
	)
}

main = () {
	ut()
	Nil
}
