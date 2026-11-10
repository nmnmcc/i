one = (s ! S(Int)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
two = (s ! S(String)) { Nil }
S(A) { S(A) }
bad = () { run(() { one() }) run(() { two() }) }
