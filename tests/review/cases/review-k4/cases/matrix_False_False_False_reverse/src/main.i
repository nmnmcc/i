S(N) = (() <- N -> Nil)
f = <N>() <- (s: S(N)) { run(() { use() }) }
run = (body) { body() }
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}

main = () { f() <- (S(() { Nil })) }
