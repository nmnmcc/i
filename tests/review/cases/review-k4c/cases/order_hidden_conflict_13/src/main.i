two = (s ! S(String)) { Nil }
S(A) { S(A) }
run(N) = (body: () -> Nil ! N) { body() }
bad = () { run(() { one() }) run(() { two() }) }
one = (s ! S(Int)) { Nil }
