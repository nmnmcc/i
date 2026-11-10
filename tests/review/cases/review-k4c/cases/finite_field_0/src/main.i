S(N) { S(f: () -> Nil ! N) }
use(N) = (s ! S(N)) { s.f() }
run(N) = (body: () -> Nil ! N) { body() }
helper(N) = () { run(() { use() }) }
