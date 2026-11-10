Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
@main
main = () {
  ! S(Never) = S(() { get() })
  use()
  Nil
}
