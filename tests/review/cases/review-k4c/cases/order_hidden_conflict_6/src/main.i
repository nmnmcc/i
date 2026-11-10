bad = () { run(() { one() }) run(() { two() }) }
run(N) = (body: () -> Nil ! N) { body() }
S(A) { S(A) }
two = (s ! S(String)) { Nil }
one = (s ! S(Int)) { Nil }
