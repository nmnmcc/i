Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(E,N) { S(f: () -> Nil ? E ! N) }
use(E,N) = (s ! S(E,N)) { s.f() }
@main
main(E,N) = () { local2 = () { ! S(Never,Dep) = S(() { get() })
run(() -> Nil ? E ! N | S(E,N) { use() }) } run(local2) }
