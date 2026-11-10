one = (s ! S(Int)) { Nil }
S(A) { S(A) }
run(N) = (body: () -> Nil ! N) { body() }
two = (s ! S(String)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
