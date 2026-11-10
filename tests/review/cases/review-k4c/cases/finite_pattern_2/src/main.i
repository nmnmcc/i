S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
helper(N) = () { local: () -> Nil ! N | S(N) = () { run(() { use() }) } run(local) }
