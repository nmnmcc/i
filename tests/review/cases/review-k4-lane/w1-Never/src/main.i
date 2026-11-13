S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap: <N>(() <- (S(N), ..N) -> Int) <- N -> Int = (body) {
	_syn3_0: S(N) = S(f = () { 1 })
	body() <- (_syn3_0)
}

main = () { assert(wrap(() { use() }) == 1) }
