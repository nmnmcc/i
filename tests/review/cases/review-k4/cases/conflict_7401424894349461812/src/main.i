S(A) { S(value: A) }
read = (s ! S(String)) { s.value <> "!" }
read_int = (s ! S(Int)) { s.value + 1 }
run = (f) { f() }
join(N, M) = (f: () -> String ! N, g: () -> Int ! M) { run(f) run(g) }
bad = () { h = () { join(read, read_int) } h() }
