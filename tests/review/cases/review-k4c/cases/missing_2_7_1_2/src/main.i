Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ^ E ! N) { body() }
S(E,N) { S(f: () -> Nil ^ E ! N) }
use(E,N) = (s ! S(E,N)) { s.f() }

main(E,N) = () { local2 = () { ! S(Never,Dep) = S(() { get() })
local = () { inner = () { use() } inner() } run(local) } run(local2) }
