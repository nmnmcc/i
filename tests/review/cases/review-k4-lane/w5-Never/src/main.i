S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(body, svc: S(N)) { body() <- (svc) }

main = () { assert(wrap(() { use() }, S(f = () { 1 })) == 1) }
