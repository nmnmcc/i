S(A) { S(f: (A) -> Int) }
use(A) = (x: A, s ! S(A)) { s.f(x) }
wrap(A) = (x: A) { local = () { use(x) } local() }

@main
main = () { ! S(Int) = S((x: Int) { x + 1 }) wrap(1) Nil }
