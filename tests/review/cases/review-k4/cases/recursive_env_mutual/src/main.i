S(N) = (() <- N -> Nil, (N) -> Nil)
f : <N>() <- (Fail(Never), ..N) -> Nil = () <- (s : S(N)) -> Nil { g() }
g = () { f() }
