Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ^ E ! N) { body() }
S(N) { S(f: () -> Nil ! N) }
use(N) = (s ! S(N)) { s.f() }
helper(N) = () { ! S(Dep) = S(f: () { get() })
run(() -> Nil ! N | S(N) { use() }) }

main = () { helper() }
