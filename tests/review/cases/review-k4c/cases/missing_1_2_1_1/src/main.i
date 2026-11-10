Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(N) { S(f: () -> Nil ! N, mark: (N) -> Nil) }
use(N) = (s ! S(N)) { s.f() }
helper(N) = () { ! S(Dep) = S(() { get() }, (_: Dep) { Nil })
local = () { use() } local() }
@main
main = () { helper() }
