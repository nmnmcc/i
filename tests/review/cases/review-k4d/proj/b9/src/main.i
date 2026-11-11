Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
run(A, N) = (body: () -> A ! N) { body() }

u1(N) = (s ! S(N)) -> Int ! S(N) | N { use() }
u2(N) = () -> Int ! S(N) | N | Dep { use() }
u3(N) = () -> Int ! S(N) | N { use() + get() }

main = () {
  ! S(Dep) = S(() { get() })
  u1() + u2() + u3()
  Nil
}
