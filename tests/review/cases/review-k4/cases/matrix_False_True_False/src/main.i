S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N))  { S(f) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
f(N) = (s ! S(N))  { run(() { use() }) }
@main
main = () { ! S(Never) = S(() { Nil }) f() }
