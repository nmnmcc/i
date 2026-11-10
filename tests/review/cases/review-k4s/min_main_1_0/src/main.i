S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
@main
main(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { use() } local() }
