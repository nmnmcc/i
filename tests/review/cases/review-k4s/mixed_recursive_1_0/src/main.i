S(N) { S(() -> Int ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
D { D }
run(N) = (x: N, f: () -> Int ! N) { f() }
bad(N) = (x: N, s ! S(N))  { run(x, () { use() }) }

