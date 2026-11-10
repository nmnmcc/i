S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { s.f() }
fw = () { use() }
