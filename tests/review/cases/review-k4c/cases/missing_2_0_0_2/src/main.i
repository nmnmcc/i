Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(E,N) { S(f: () -> Nil ? E ! N) }
use(E,N) = (s ! S(E,N)) { s.f() }
@main
main(E,N) = () { local2 = () { use() } run(local2) }
