S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(A,N) = (f: () -> A ! N) { f() }

main(N) = (s ! S(N)) { run(() { use() }) }
