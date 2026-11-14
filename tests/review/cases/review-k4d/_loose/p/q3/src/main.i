S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) { s.f() }
run = <A, N>(body : () <- N -> A) { body() }
fw = () { run(() { use() }) }
