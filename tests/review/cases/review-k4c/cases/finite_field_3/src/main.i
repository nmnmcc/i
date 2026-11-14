S(N) = (f = () <- N -> Nil)
use = <N>() <- (s : S(N)) { s.f() }
run = <N>(body : () <- N -> Nil) { body() }
helper = <N>() { run(() -> Nil { use() }) }
