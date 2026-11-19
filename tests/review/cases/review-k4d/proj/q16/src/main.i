S(N) = (f = () <- N -> Int, m = (N) -> ())
use = <N>() <- (s : S(N)) { s.f() }
run = <N>(body : () <- N -> Int) { body() }
fw = <N>() <- (s : S(N)) -> Int { run(() { use() }) }
