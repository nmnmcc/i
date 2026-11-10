Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
T(N) { T(f: () -> Int ! N, m: (N) -> Nil) }
us(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
ut(N) = (t ! T(N)) {
  T(f, ..) = t
  f()
}
usa(N) = (s ! S(N)) -> Int ! N | S(N) {
  S(f, ..) = s
  f()
}
run(A, N) = (body: () -> A ! N) { body() }
h0 = () { run(() { us() }) + ut() }
h1 = () { usa() }
@main
main = () {
  ! Dep = Dep(1)
  ! T(Dep) = T(() { get() }, (_: Dep) { Nil })
  l = () { run(() { us() + usa() }) }
  ! S(Dep | T(Dep)) = S(() { ut() + get() }, (_: Dep | T(Dep)) { Nil })
  h0() + l() + ut() + h1()
  Nil
}
