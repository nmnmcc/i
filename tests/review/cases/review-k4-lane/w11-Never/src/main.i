S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap: <N>(() <- (S(N), ..N) -> Int, S(N)) <- N -> Int = (body, svc) {
	body() <- (svc)
}

main = () {
	assert(wrap(() { use() }, S(f = () { 1 }, m = (_: Never) { Nil })) == 1)
}
