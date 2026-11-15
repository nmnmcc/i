S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) -> Int { s.f() }
fw = () { use() }
