S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = (f) { f() }
id = (x) { x }
f = () -> Nil {
	f = (use, use)
	f.0()
	f.1()
}

main = () { f() <- (S(() { Nil }, (_: Never) { Nil })) }
