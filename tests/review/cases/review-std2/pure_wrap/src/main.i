S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
wrap = <A, N>(f: () <- N -> A) { f() }
example = () { wrap(() { use() }) <- (S(() { Nil }, (_: Never) { Nil })) }
