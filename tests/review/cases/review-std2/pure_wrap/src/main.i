S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
wrap(A,N) = (f: () -> A ! N) { f() }
example = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) wrap(() { use() }) }
