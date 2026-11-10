one = (s ! S(Int)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
bad = () { run(() { one() }) run(() { two() }) }
two = (s ! S(String)) { Nil }
S(A) { S(A) }
