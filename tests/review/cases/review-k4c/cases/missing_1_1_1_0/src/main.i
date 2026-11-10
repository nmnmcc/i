Dep { Dep }
get = (d ! Dep) { Nil }
run(E,N) = (body: () -> Nil ? E ! N) { body() }
S(N) { S(f: () -> Nil ! N, mark: (N) -> Nil) }
use(N) = (s ! S(N)) { s.f() }
@main
main(N) = () { ! S(Dep) = S(() { get() }, (_: Dep) { Nil })
run(() { use() }) }
