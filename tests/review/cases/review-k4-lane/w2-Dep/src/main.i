Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(body: () <- (S(N), ..N) -> Int) {
	_syn3_0: S(N) = S(f = () { 1 })
	body() <- (_syn3_0)
}

main = () { assert(wrap(() { use() }) == 1) <- (Dep(value = 7)) }
