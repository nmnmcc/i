S(A) { S(f: (A) -> Nil) }
use(A) = (x: A, s ! S(A)) { s.f(x) }
wrap(A) = (x: A) { local = () { inner = () { use(x) } inner() } local() }
