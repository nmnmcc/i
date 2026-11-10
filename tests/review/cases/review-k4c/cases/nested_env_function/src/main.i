Dep { Dep(Int) }
get = (d ! Dep) { Dep(x) = d x }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
make(N) = (f: () -> Int ! N) { inner = (g) { local = () { g(f) } local() } inner }
run(N) = (f: () -> Int ! N) { f() }
@main
main = () { ! Dep = Dep(11) ! S(Dep) = S(() { get() }) wrapped = make(() { use() }) assert(wrapped(run) == 11) Nil }
