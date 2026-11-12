S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
make = (f: () -> Nil) { () { use() <- (S(f, (_: Never) { Nil })) } }

main = () {
	f = make(() { Nil })
	f()
}
