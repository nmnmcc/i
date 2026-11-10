S(A) { S(value: A) }
use(A, N) = (f: () -> Nil ! N, s ! S(A)) { f() s.value }
two = (s ! S(String)) { Nil }
bad = () -> Int ! S(Int) { use(two) }
