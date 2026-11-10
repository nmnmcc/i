S(N) { S(() -> Nil ! N, (N) -> Nil) }
f(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }
run = (body) { body() }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f, _) = s f() }
@main
main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
