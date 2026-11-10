S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
forward(N) = (s ! S(N)) -> Nil ! N | S(N) { local: () -> Nil ! N | S(N) = () { use() } run(local) }

example = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) forward() }
