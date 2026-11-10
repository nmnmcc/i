S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
f(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }
@main
main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
