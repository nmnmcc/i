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
h0 = () { run(() { us() }) + ut() }
h1 = () { usa() }

main = () {
  ! S(S(Never)) = S(f: () { run(() { ut() + get() }) }, m: (_: S(Never)) { Nil })
  ! Dep = Dep(v: 1)
  ! T(Dep) = T(f: () { get() }, m: (_: Dep) { Nil })
  l = () { run(() { us() + usa() }) }
  h0() + l() + ut() + h1()
  Nil
}
