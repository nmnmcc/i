run(N) = (body: () -> Nil ! N) { body() }
S(A) { S(A) }
two = (s ! S(String)) { Nil }
bad = () { run(() { one() }) run(() { two() }) }
one = (s ! S(Int)) { Nil }
