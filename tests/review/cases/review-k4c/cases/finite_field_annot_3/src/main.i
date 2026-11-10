S(N) { S(f: () -> Nil ! N) }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { s.f() }
run(N) = (body: () -> Nil ! N) { body() }
helper(N) = () { run(() -> Nil ! N | S(N) { use() }) }
