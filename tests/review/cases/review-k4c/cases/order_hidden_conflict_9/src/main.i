S(A) { S(A) }
bad = () { run(() { one() }) run(() { two() }) }
run(N) = (body: () -> Nil ! N) { body() }
one = (s ! S(Int)) { Nil }
two = (s ! S(String)) { Nil }
