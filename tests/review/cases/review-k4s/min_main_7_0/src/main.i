S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(A,N) = (f: () -> A ! N) { f() }

main(N) = () { local: () -> Nil ! N | S(N) = () { use() } run(local) }
