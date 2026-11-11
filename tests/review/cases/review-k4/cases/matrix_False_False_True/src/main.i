S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f) = s f() }
run = (body) { body() }
f(N) = (s ! S(N)) -> Nil ! N | S(N) { run(() { use() }) }

main = () { ! S(Never) = S(() { Nil }) f() }
