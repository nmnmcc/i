S(N) { S(() -> Nil ! N, (N) -> Nil) }
f(N) = (s ! S(N)) -> Nil ! N { g() }
g = () { f() }
