S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
forward(N) = (s ! S(N)) { run(() { use() }) }
f = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) forward() }
