S(N) = (() <- N -> Nil)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
closed : () <- (S(Never)) -> Nil = use

example = () { closed() <- (S(() { Nil })) }
