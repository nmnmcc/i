S(A) { S(f: (A) -> Int) }
use(A) = (x: A, s ! S(A)) { s.f(x) }
wrap(A) = (x: A) { local = () { use(x) } local() }
different = (f: (Int) -> Int ! S(String)) { f }
test = different(wrap)
