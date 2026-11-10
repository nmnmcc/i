S(N) { S(() -> Nil ! N, (N) -> Nil) }
f(N) = (s ! S(N))  { run(() { use() }) }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N))  { S(f, _) = s f() }
@main
main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
