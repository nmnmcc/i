S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(svc: S(N)) { use() <- (svc) }
