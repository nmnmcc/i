S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { s.f() }
run(A, N) = (body: () -> A ! N) { body() }
fw(N) = () -> Int ! N | S(N) { run(() { use() }) }
