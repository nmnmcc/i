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
h0 = () { us() }
h1 = () { run(h0) + run(() { ut() }) }

main = () {
  ! S(Never) = S(f: () { 1 }, m: (_: Never) { Nil })
  ! T(Dep | S(Dep)) = T(f: () { us() + h0() }, m: (_: Dep | S(Dep)) { Nil })
  k = () { h1() }
  k() + ut()
  Nil
}
