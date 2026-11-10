S(N) { S(() -> Nil ! N) }
f(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }
run = (body) { body() }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f) = s f() }
@main
main = () { ! S(Never) = S(() { Nil }) f() }
