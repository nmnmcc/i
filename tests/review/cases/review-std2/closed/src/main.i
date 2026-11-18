S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
closed : () <- (S(Never)) -> () = use

example = () { closed() <- (S(() { () })) }
