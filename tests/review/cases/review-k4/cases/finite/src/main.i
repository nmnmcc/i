S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
forward(N) = (s ! S(N)) { run(() { use() }) }
example = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) forward() }

main = () { example() }
