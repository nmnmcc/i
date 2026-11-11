Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
run(A, N) = (body: () -> A ! N) { body() }


main = () {
  ! S(Dep) = S(() { get() })
  use()
  Nil
}
