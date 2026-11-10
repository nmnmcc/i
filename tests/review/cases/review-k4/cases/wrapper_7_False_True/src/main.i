S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run = (f) { f() }
id = (x) { x }
f(N) = (s ! S(N))  { f = () { use() } run(f) run(f) }
@main
main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
