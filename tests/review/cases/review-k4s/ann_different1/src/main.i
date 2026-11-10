S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
D { D }
test(N,M) = (f: () -> Nil ! N, g: () -> Nil ! M) -> Nil ! N | M { f() g() }
