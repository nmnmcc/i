S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = (body) { body() }
f = <N>() <- (s: S(N)) { run(() { use() }) }

main = () { f() <- (S(() { Nil })) }
