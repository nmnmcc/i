S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
make(N) = (s: S(N)) { () { ! S(N) = s use() } }
@main
main = () { f = make(S(() { Nil }, (_: Never) { Nil })) f() }
