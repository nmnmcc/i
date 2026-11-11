S(N) { S(() -> Nil ! N, (N) -> Nil) }
f(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f, _) = s f() }

main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
