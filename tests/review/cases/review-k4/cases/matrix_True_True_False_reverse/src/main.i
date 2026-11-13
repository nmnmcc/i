S(N) = (() <- N -> Nil, (N) -> Nil)
f = <N>() <- (s: S(N)) { run(() { use() }) }
run = <N>(body: () <- N -> Nil) { body() }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}

main = () { f() <- (S(() { Nil }, (_: Never) { Nil })) }
