S(N) = (f = () <- N -> Nil)
use = <N>() <- (s : S(N)) -> Nil { s.f() }
run = <N>(body : () <- N -> Nil) { body() }
helper = () -> Nil { run(() { use() }) }
