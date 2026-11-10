S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (f: () -> Nil ! N) { f() }
bad(N) = () { ! S(N) = S(() { use() }, (_: N) { Nil }) local = () { use() } local() }
