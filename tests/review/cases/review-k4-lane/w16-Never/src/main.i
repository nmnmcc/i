S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (svc) {
	_syn3_0: S(_) = svc
	use() <- (_syn3_0)
}

main = () { assert(wrap(S(f = () { 1 }, m = (_: Never) { Nil })) == 1) }
