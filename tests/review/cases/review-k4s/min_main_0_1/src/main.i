S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
@main
main(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { use() } run(local) }
