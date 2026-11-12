S(A) { S(value: A) }
one = (s ! S(Int)) { s.value + 1 }
two = (s ! S(String)) { s.value <> "!" }
run(N) = (x: N, f: () -> Int ! N) { f() }
bad = () { run(S(value: "hi"), () { one() }) }

