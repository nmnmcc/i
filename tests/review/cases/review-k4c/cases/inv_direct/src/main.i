S(A) { S(f: (A) -> A) }
use(A) = (x: A, s ! S(A)) { s.f(x) }
wrap(A) = (x: A) { use(x) }
