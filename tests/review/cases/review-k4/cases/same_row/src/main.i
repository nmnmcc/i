S(N) { S(() -> Nil ! N, (N) -> Nil) }
T(N) { T(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N), t ! T(N)) { S(f, _) = s T(g, _) = t f() g() }
