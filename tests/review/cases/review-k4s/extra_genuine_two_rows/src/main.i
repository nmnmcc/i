S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
T(N) { T(() -> Nil ! N, (N) -> Nil) }
f(N) = (s ! S(N)) -> Nil ! N { g() }
g(N) = (t ! T(N)) -> Nil ! N { f() }
