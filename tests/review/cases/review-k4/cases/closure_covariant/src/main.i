S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
make(N) = (f: () -> Nil ! N) { () { ! S(N) = S(f) use() } }
@main
main = () { f = make(() { Nil }) f() }
