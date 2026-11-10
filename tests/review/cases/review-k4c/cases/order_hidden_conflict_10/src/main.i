S(A) { S(A) }
bad = () { run(() { one() }) run(() { two() }) }
two = (s ! S(String)) { Nil }
one = (s ! S(Int)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
