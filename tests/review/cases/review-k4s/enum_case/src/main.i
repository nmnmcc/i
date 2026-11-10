S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
D { D }
test(N) = (s ! S(N)) -> Nil ! N | S(N) { use()
use()
use()
use() }
