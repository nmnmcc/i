S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
make(N) = (f: () -> Nil ! N) { () { ! S(N) = S(f, (_: N) { Nil }) use_n: () -> Nil ! N | S(N) = use use_n() } }
@main
main = () { f = make(() { Nil }) f() }
