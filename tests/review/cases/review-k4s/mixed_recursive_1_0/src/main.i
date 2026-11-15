S(N) = (() <- N -> Int, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
D = ()
run = <N>(x : N, f : () <- N -> Int) { f() }
bad = <N>(x : N) <- (s : S(N)) { run(x, () { use() }) }
