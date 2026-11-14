S(A) = (value = A)
one = () <- (s : S(Int)) { s.value + 1 }
two = () <- (s : S(String)) { s.value <> "!" }
run = <N>(f : () <- N -> Int, x : N) { f() }
bad = () { run(() { one() }, S(value = "hi")) }
