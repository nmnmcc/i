S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(svc: S(N)) -> Int { use() <- (svc) }

main = () { assert(wrap(S(f = () { 1 }, m = (_: Never) { Nil })) == 1) }
