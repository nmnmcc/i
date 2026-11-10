S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
helper(N) = () { run(() -> Nil ! N | S(N) { use() }) }
