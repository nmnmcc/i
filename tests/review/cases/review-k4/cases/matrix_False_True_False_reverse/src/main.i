S(N) = (() <- N -> Nil)
f = <N>() <- (s: S(N)) { run(() { use() }) }
run = <N>(body: () <- N -> Nil) { body() }
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}

main = () { f() <- (S(() { Nil })) }
