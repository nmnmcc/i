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
h0 = () { get() + us() }
h1 = () {
  ! Dep = Dep(v: 2)
  h0()
}

main = () {
  ! S(Dep) = S(f: () { get() }, m: (_: Dep) { Nil })
  a = h1()
  ! Dep = Dep(v: 3)
  ! T(Dep | S(Dep)) = T(f: () { h0() }, m: (_: Dep | S(Dep)) { Nil })
  a + ut() + run(() { ut() })
  Nil
}
