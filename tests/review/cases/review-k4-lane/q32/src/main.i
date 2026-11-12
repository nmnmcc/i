S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) -> Int { s.f() }

fw = () -> Int { use() }

main = () { assert(fw() == 1) <- (S(f = () { 1 })) }
