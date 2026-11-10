S(A) { S(f: () -> Nil ! A) }
use(A) = (s ! S(A)) { s.f() }
wrap(A) = (x: A, f) { local = () { f() } use() local() }
