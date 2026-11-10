one = (s ! S(Int)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
bad = () { run(() { one() }) run(() { two() }) }
S(A) { S(A) }
two = (s ! S(String)) { Nil }
