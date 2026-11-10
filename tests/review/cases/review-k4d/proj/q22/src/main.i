S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { s.f() }
fw(N) = () -> Int ! S(N) | N { use() }
