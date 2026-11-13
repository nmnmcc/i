Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }

bad: <N>() <- (Fail(Never), S(N)) -> Int = () <- (s: S(N)) -> Int {
	S(f) = s
	f()
}

main = () {
	_syn3_0: S(Dep) = S(() { get() })
	{
		bad()
		Nil
	} <- (_syn3_0)
}
