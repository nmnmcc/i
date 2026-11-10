S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (body: () -> A ! N) { body() }
forward(N) = (s ! S(N)) { run(() { use() }) }

