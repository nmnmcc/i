Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(body: () <- (S(N), ..N) -> Int) -> Int {
	body() <- (S(f = () { 1 }, m = (_: N) { Nil }))
}

main = () { assert(wrap(() { use() }) == 1) <- (Dep(value = 7)) }
