S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

fw = <N>() <- (s: S(N)) -> Int { use() }

main = () {
	_syn3_0: S(Never) = S(f = () { 1 })
	assert(fw() == 1) <- (_syn3_0)
}
