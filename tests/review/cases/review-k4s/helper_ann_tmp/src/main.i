S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
D { D }
run(N): (body: () -> Nil ! N) -> Nil ! N = (body) { body() }
forward(N) = (s ! S(N)) -> Nil ! N | S(N) { local = run local(() { use() }) }
