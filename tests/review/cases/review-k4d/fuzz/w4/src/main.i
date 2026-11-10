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
h0 = () {
  ! S(T(T(T(Never)))) = S(() { 1 + us() + usa() }, (_: T(T(T(Never)))) { Nil })
  ! T(Never) = T(() { run(() { 2 + 2 }) }, (_: Never) { Nil })
  ut()
}
h1 = () {
  l0_0 = () {
    ! S(Dep) = S(() { get() + ut() }, (_: Dep) { Nil })
    get()
  }
  ! S(Never) = S(() { get() }, (_: Never) { Nil })
  ! T(S(Never)) = T(() { 1 + run(() { h0() + 1 }) + ut() }, (_: S(Never)) { Nil })
  l0_0() + get()
}
h2 = () {
  l0_0 = () {
    l1_0 = () {
      ! T(S(Dep)) = T(() { ut() + us() }, (_: S(Dep)) { Nil })
      ! S(Dep) = S(() { usa() + usa() + 1 }, (_: Dep) { Nil })
      h0() + ut()
    }
    ! S(S(Never)) = S(() { l1_0() + 1 + us() }, (_: S(Never)) { Nil })
    usa() + h1()
  }
  ! Dep = Dep(1)
  ! T(Never) = T(() { run(() { 2 + 2 + get() }) }, (_: Never) { Nil })
  run(() { get() + l0_0() + us() }) + run(() { us() + 1 })
}
@main
main = () {
  ut()
  Nil
}
