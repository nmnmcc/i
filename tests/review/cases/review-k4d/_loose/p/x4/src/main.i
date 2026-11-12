Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f = f, ..) = s
  f()
}

wrap(A, N) = (body: () -> A ! N, svc: S(Dep)) -> A ! N {
  ! S(Dep) = svc
  body()
}

main = () {
  ! S(Never) = S(f: () { 1 }, m: (_: Never) { Nil })
  wrap(() { use() }, S(f: () { get() }, m: (_: Dep) { Nil }))
  Nil
}

