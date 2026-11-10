S(A) { S(A) }
one = (s ! S(Int)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
run(N) = (body: () -> Nil ! N) { body() }
two = (s ! S(String)) { Nil }
