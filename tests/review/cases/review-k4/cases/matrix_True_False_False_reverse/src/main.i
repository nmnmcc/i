S(N) = (() <- N -> Nil, (N) -> Nil)
f = <N>() <- (s: S(N)) { run(() { use() }) }
run = (body) { body() }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}

main = () { f() <- (S(() { Nil }, (_: Never) { Nil })) }
