Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(() -> Int ! N) }
T(N) { T(() -> Int ! N) }
use(N) = (s ! S(N)) -> Int ! N | S(N) {
  S(f) = s
  f()
}
uset(N) = (s ! T(N)) {
  T(f) = s
  f()
}
@main
main = () {
  ! S(Dep) = S(() { get() })
  ! T(Never) = T(() { use() })
  uset()
  Nil
}
