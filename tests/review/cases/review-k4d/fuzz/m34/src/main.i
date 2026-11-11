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
h0 = () { get() + us() }
h1 = () {
  ! Dep = Dep(2)
  h0()
}

main = () {
  a = h1()
  ! T(Dep | S(Dep)) = T(() { h0() }, (_: Dep | S(Dep)) { Nil })
  ! Dep = Dep(3)
  a + ut() + run(() { ut() })
  Nil
}
