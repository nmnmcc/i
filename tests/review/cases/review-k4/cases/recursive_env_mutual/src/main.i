S(N) = (() <- N -> Nil, (N) -> Nil)
f = <N>() <- (s: S(N)) -> Nil { g() }
g = () { f() }
