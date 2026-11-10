two = (s ! S(String)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
one = (s ! S(Int)) { Nil }
S(A) { S(A) }
bad = () { run(() { one() }) run(() { two() }) }
