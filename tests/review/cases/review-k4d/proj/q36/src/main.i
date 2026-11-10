S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) -> Int ! S(N) | N { s.f() }
fw(N) = () -> Int ! N | S(N) { use() }
