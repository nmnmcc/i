Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ^ E ! N) { body() }
S(A,B) { S(f: () -> Nil ! A, g: () -> Nil ! B) }
use(A,B) = (s ! S(A,B)) { s.f() s.g() }

main(A,B) = () { local2 = () { ! S(Dep,Dep) = S(() { get() }, () { get() })
local = () { use() } local() } run(local2) }
