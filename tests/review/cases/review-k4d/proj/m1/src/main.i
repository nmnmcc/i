Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
T(N) { T(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  s.f()
}
uset(N) = (s ! T(N)) {
  s.f()
}

main = () {
  ! S(Dep) = S(f: () { get() }, m: (_: Dep) { Nil })
  ! T(S(Dep)) = T(f: () { use() }, m: (_: S(Dep)) { Nil })
  uset()
  Nil
}
