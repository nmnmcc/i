S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
D = ()
forward = <N>() <- (s: S(N)) { run(() { use() }) }
f = () { forward() <- (D, S(() <- (d: D) { Nil }, (_: D) { Nil })) }
