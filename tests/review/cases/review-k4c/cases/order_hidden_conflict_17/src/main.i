bad = () { run(() { one() }) run(() { two() }) }
one = (s ! S(Int)) { Nil }
two = (s ! S(String)) { Nil }
run(N) = (body: () -> Nil ! N) { body() }
S(A) { S(A) }
