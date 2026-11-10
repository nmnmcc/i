D { D(Int) }
read = (d ! D) { D(n) = d n }
S(A,B) { S(() -> Int ! A, () -> Int ! B) }
use(A,B) = (s ! S(A,B)) { S(f,g) = s f() + g() }
run(N) = (f: () -> Int ! N) { f() }
wrap(A,B) = () { local: () -> Int ! A | B | S(A,B) = () { run(() { use() }) } run(local) }
@main
main = () { ! D = D(7) ! S(D,Never) = S(() { read() }, () { 3 }) assert(wrap() == 10) Nil }
