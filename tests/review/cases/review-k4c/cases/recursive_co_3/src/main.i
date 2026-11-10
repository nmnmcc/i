S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (f: () -> Nil ! N) { f() }
bad(N) = () { ! S(N) = S(() { use() }) local = () { use() } local() }
