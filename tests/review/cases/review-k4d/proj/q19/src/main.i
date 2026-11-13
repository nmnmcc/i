S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) { s.f() }
run = <N>(body: () <- N -> Int) { body() }
fw = () -> Int { use() }
