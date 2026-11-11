Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ^ E ! N) { body() }
S(N) { S(f: () -> Nil ! N) }
use(N) = (s ! S(N)) { s.f() }

main(N) = () { local2 = () { local = () { inner = () { use() } inner() } run(local) } run(local2) }
