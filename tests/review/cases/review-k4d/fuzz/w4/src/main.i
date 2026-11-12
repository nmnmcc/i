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
h0 = () {
  ! S(T(T(T(Never)))) = S(f: () { 1 + us() + usa() }, m: (_: T(T(T(Never)))) { Nil })
  ! T(Never) = T(f: () { run(() { 2 + 2 }) }, m: (_: Never) { Nil })
  ut()
}
h1 = () {
  l0_0 = () {
    ! S(Dep) = S(f: () { get() + ut() }, m: (_: Dep) { Nil })
    get()
  }
  ! S(Never) = S(f: () { get() }, m: (_: Never) { Nil })
  ! T(S(Never)) = T(f: () { 1 + run(() { h0() + 1 }) + ut() }, m: (_: S(Never)) { Nil })
  l0_0() + get()
}
h2 = () {
  l0_0 = () {
    l1_0 = () {
      ! T(S(Dep)) = T(f: () { ut() + us() }, m: (_: S(Dep)) { Nil })
      ! S(Dep) = S(f: () { usa() + usa() + 1 }, m: (_: Dep) { Nil })
      h0() + ut()
    }
    ! S(S(Never)) = S(f: () { l1_0() + 1 + us() }, m: (_: S(Never)) { Nil })
    usa() + h1()
  }
  ! Dep = Dep(v: 1)
  ! T(Never) = T(f: () { run(() { 2 + 2 + get() }) }, m: (_: Never) { Nil })
  run(() { get() + l0_0() + us() }) + run(() { us() + 1 })
}

main = () {
  ut()
  Nil
}
