S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
example = () { use() <- (S(() { Nil }, (_ : Never) { Nil })) }
