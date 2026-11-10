Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(N) { S(f: () -> Nil ! N, mark: (N) -> Nil) }
use(N) = (s ! S(N)) { s.f() }
@main
main(N) = () { local2 = () { ! S(Dep) = S(() { get() }, (_: Dep) { Nil })
local = () { inner = () { use() } inner() } run(local) } run(local2) }
