S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
make = <N>(s: S(N)) { () { use() <- (s) } }

main = () {
	f = make(S(() { Nil }, (_: Never) { Nil }))
	f()
}
