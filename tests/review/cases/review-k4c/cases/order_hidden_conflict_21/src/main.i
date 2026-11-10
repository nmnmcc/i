S(A) { S(A) }
run(N) = (body: () -> Nil ! N) { body() }
one = (s ! S(Int)) { Nil }
two = (s ! S(String)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
