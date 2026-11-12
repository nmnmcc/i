S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(f: () <- N -> Nil) { f() }
bad = <N>() {
	{
		local = () { use() }
		local()
	} <- (S(() { use() }))
}
