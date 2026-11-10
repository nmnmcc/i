S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
first(N) = (s ! S(N)) -> Nil ! N { second() }
second(N) = (s ! S(N)) -> Nil ! N { first() use() }
