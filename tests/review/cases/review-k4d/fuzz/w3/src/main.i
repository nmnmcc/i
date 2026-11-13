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
h0 = () { 1 }

main = () {
	_syn3_0 : S(Dep | S(Dep)) = S(
		f = () { run(() { h0() + 1 + get() }) },
		m = (_ : { Dep | S(Dep) }) { Nil },
	)
	{
		run(() { run(() { usa() + h0() }) + 1 })
		Nil
	} <- (_syn3_0)
}
