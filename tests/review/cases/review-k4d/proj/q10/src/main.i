S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { s.f() }
run(N) = (body: () -> Int ! N) { body() }
fw(N) = () -> Int ! N | S(N) { h = () { use() }
 h() }
