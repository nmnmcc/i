S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
wrap(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }
