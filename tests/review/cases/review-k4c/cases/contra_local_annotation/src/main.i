S(A) { S(f: (A) -> Int) }
use(A) = (x: A, s ! S(A)) { s.f(x) }
wrap(A) = (x: A) { local: () -> Int ! S(A) = () { use(x) } local() }


main = () { ! S(String) = S(f: (_: String) { 42 }) wrap(1) Nil }
