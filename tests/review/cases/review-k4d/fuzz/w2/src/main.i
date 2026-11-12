Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
T(N) { T(f: () -> Int ! N, m: (N) -> Nil) }
us(N) = (s ! S(N)) {
  S(f = f, ..) = s
  f()
}
ut(N) = (t ! T(N)) {
  T(f = f, ..) = t
  f()
}
usa(N) = (s ! S(N)) -> Int ! N | S(N) {
  S(f = f, ..) = s
  f()
}
run(A, N) = (body: () -> A ! N) { body() }

main = () {
  l0_0 = () {
    ut()
  }
  ! T(T(S(Never))) = T(f: () { get() + ut() + us() }, m: (_: T(S(Never))) { Nil })
  ut()
  Nil
}
