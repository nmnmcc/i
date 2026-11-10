Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(N) { S(f: () -> Nil ! N) }
use(N) = (s ! S(N)) { s.f() }
@main
main(N) = () { local2 = () { ! S(Dep) = S(() { get() })
local: () -> Nil ! N | S(N) = () { run(() { use() }) } run(local) } run(local2) }
