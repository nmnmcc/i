S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
@main
main(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { use() } Nil }
