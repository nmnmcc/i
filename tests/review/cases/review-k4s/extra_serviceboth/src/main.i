S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
forward(N): () -> Nil ! N | S(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }
