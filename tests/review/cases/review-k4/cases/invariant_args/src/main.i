S(A, N) { S(A, (A) -> Nil, () -> Nil ! N, (N) -> Nil) }
use(A, N) = (s ! S(A, N)) { S(x, f, g, _) = s f(x) g() x }
run = (f) { f() }
f(A, N) = (s ! S(A, N)) { run(() { use() }) }
