Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ^ E ! N) { body() }
S(E,N) { S(f: () -> Nil ^ E ! N) }
use(E,N) = (s ! S(E,N)) { s.f() }
helper(E,N) = () { ! S(Never,Dep) = S(() { get() })
local: () -> Nil ^ E ! N | S(E,N) = () { run(() { use() }) } run(local) }

main = () { helper() }
