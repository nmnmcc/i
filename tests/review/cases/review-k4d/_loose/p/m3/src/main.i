Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N) }
T(N) { T(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  s.f()
}
uset(N) = (s ! T(N)) {
  s.f()
}

main = () {
  ! S(Dep) = S(() { get() })
  ! T(S(Dep)) = T(() { use() }, (_: S(Dep)) { Nil })
  uset()
  Nil
}
