S(A) { S(value: A) }
one = (s ! S(Int)) { s.value + 1 }
two = (s ! S(String)) { s.value <> "!" }
run(N) = (f: () -> Int ! N, x: N) { f() }
bad = () { run(() { one() }, S(value: "hi")) }

