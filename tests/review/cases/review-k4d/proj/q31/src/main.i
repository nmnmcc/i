S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) -> Int ! N | S(N) { s.f() }
fw(N) = () -> Int ! N | S(N) { use() }
