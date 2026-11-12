S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
Dep { Dep(value: Int) }
get = (d ! Dep) { d.value }
make = (n: Int) {
 () {
   ! Dep = Dep(value: n)
   ! S(Dep) = S(() { get() Nil }, (_: Dep) { Nil })
   use()
 }
}
example = () { f = make(7) f() }
