S(N) = (f = () <- N -> Int)

use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(svc : S(N)) {
	_syn3_0 : S(N) = svc
	use() <- (_syn3_0)
}

main = () { assert(wrap(S(f = () { 1 })) == 1) }
