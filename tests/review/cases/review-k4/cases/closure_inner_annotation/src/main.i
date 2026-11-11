S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
make(N) = (f: () -> Nil ! N) { () -> Nil ! N { ! S(N) = S(f, (_: N) { Nil }) use() } }

main = () { f = make(() { Nil }) f() }
