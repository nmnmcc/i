S(N) = (() <- N -> Nil)
f = <N>() <- (s: S(N)) -> Nil { run(() { use() }) }
run = (body) { body() }
use = <N>() <- (s: S(N)) -> Nil {
	S(f) = s
	f()
}

main = () { f() <- (S(() { Nil })) }
