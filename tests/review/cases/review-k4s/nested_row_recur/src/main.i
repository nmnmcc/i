S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
bad(N) = (s ! S(N)) -> Nil ! S(N) { S(f, _) = s f() }
