S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

fw = () { use() }

main = () { assert(fw() == 1) <- (S(f = () { 1 })) }
