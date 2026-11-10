Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(A,B) { S(f: () -> Nil ! A, g: () -> Nil ! B) }
use(A,B) = (s ! S(A,B)) { s.f() s.g() }
helper(A,B) = () { local: () -> Nil ! A | B | S(A,B) = () { run(() { use() }) } run(local) }
@main
main = () { helper() }
