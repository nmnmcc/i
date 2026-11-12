S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(svc: S(N)) -> Int { use() <- (svc) }

main = () { assert(wrap(S(f = () { 1 })) == 1) }
