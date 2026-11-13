S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <N>(f: () <- N -> Nil) { f() }
bad = <N>() {
	local = () { use() <- (S(() { use() }, (_: N) { Nil })) }
	local()
}
