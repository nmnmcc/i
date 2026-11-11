S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
make = (f: () -> Nil) { () { ! S(Never) = S(f, (_: Never) { Nil }) use() } }

main = () { f = make(() { Nil }) f() }
