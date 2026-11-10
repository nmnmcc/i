S(A) { S(A) }
two = (s ! S(String)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
run(N) = (body: () -> Nil ! N) { body() }
one = (s ! S(Int)) { Nil }
