S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { s.f() }
run(N) = (body: () -> Int ! N) { body() }
fw(N) = (s ! S(N)) -> Int ! N | S(N) { run(() { use() }) }
