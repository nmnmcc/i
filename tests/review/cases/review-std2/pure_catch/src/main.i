(i/fail)
S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
example = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) fail.catch(() { use() }) }
