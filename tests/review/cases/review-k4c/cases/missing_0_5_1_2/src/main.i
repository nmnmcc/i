Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ^ E ! N) { body() }
S(N) { S(f: () -> Nil ! N) }
use(N) = (s ! S(N)) { s.f() }

main(N) = () { local2 = () { ! S(Dep) = S(f: () { get() })
run(() -> Nil ! N | S(N) { use() }) } run(local2) }
