S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) { s.f() }
run = <N>(body : () <- N -> Int) { body() }
fw = () -> Int { run(() { use() }) }
