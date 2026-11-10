S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
D { D }
forward(N) = (s ! S(N)) { run(() { use() }) }
f = () { ! D = D ! S(D) = S((d ! D) { Nil }, (_: D) { Nil }) forward() }
