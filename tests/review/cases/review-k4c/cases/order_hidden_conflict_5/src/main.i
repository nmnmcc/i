S(A) { S(A) }
one = (s ! S(Int)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
two = (s ! S(String)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
