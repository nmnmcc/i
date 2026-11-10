S(N) { S(() -> Nil ! N, (N) -> Nil) }
T(N) { T(() -> Nil ! N, (N) -> Nil) }
use(N, M) = (s ! S(N), t ! T(M)) { S(f, _) = s T(g, _) = t f() g() }
