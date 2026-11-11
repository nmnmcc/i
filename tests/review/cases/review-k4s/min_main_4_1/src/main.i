S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }

main(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { Nil } local() }
