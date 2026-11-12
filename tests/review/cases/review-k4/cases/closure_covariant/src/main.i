S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
make = <N>(f: () <- N -> Nil) { () { use() <- (S(f)) } }

main = () {
	f = make(() { Nil })
	f()
}
