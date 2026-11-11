S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run = (f) { f() }
id = (x) { x }
f(N) = (s ! S(N)) -> Nil ! N | S(N) { f = (use, use) f.0() f.1() }

main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
