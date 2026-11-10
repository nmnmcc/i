run(N) = (body: () -> Nil ! N) { body() }
one = (s ! S(Int)) { Nil }
S(A) { S(A) }
bad = () { run(() { one() }) run(() { two() }) }
two = (s ! S(String)) { Nil }
