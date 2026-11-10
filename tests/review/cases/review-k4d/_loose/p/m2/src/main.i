Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
T(N) { T(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  s.f()
}
uset(N) = (s ! T(N)) {
  s.f()
}
@main
main = () {
  ! S(Dep) = S(() { get() }, (_: Dep) { Nil })
  ! T(S(Dep)) = T(() { use() })
  uset()
  Nil
}
