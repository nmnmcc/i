two = (s ! S(String)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
bad = () { run(() { one() }) run(() { two() }) }
S(A) { S(A) }
one = (s ! S(Int)) { Nil }
