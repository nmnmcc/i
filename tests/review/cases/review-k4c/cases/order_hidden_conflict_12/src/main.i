bad = () { run(() { one() }) run(() { two() }) }
S(A) { S(A) }
two = (s ! S(String)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
one = (s ! S(Int)) { Nil }
