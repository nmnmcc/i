S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(body: () <- (S(N), ..N) -> Int, svc: S(N)) -> Int { body() <- (svc) }

main = () {
	assert(wrap(() { use() }, S(f = () { 1 }, m = (_: Never) { Nil })) == 1)
}
