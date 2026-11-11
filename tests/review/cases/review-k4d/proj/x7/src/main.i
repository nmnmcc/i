Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}

wrap(N) = (body: () -> Int ! N, svc: S(Dep)) -> Int ! N {
  body()
  ! S(Dep) = svc
  body()
}

main = () {
  ! S(Never) = S(() { 1 }, (_: Never) { Nil })
  wrap(() { use() }, S(() { get() }, (_: Dep) { Nil }))
  Nil
}

