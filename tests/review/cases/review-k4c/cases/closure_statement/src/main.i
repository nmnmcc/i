Dep { Dep(value: Int) }
get = (d ! Dep) { d.value }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (f: () -> Int ! N) { f() }

main = () { local = () { ! S(Dep) = S(() { get() }) run(() { use() }) } local() }
