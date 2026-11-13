S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) -> Nil {
	S(f, _) = s
	f()
}
run = (body) { body() }
f = <N>() <- (s: S(N)) -> Nil { run(() { use() }) }

main = () { f() <- (S(() { Nil }, (_: Never) { Nil })) }
