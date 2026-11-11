S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N))  { S(f, _) = s f() }
run = (body) { body() }
f(N) = (s ! S(N))  { run(() { use() }) }

main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
