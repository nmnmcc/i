S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
example = () { use() <- (S(() { () }, (_ : Never) { () })) }
