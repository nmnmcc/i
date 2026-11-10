bad = () { run(() { one() }) run(() { two() }) }
two = (s ! S(String)) { Nil }
one = (s ! S(Int)) { Nil }
S(A) { S(A) }
run(N) = (body: () -> Nil ! N) { body() }
