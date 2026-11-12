S(N) = (() <- N -> Nil, (N) -> Nil)
f = <N>() <- (s: S(N)) -> Nil { run(() { use() }) }
run = (body) { body() }
use = <N>() <- (s: S(N)) -> Nil {
	S(f, _) = s
	f()
}

main = () { f() <- (S(() { Nil }, (_: Never) { Nil })) }
