S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body: () <- N -> Nil) { body() }
example = () { run(() { use() }) <- (S(() { Nil }, (_: Never) { Nil })) }
