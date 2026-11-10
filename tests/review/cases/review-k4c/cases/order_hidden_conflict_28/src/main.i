two = (s ! S(String)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
S(A) { S(A) }
one = (s ! S(Int)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
