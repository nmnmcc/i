S(A) = (value = A)
one = () <- (s: S(Int)) { s.value + 1 }
two = () <- (s: S(String)) { s.value <> "!" }
run = <N>(x: N, f: () <- N -> Int) { f() }
bad = () { run(S(value = "hi"), () { one() }) }
