S(N) { S(() -> Nil ! N) }
f(N) = (s ! S(N))  { run(() { use() }) }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N))  { S(f) = s f() }

main = () { ! S(Never) = S(() { Nil }) f() }
