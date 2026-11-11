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

main = () {
  w = () {
    ! Dep = Dep(1)
    ut() + run(() { us() })
  }
  ! T(Dep) = T(() { get() }, (_: Dep) { Nil })
  ! S(Dep | T(Dep)) = S(() { ut() + get() }, (_: Dep | T(Dep)) { Nil })
  l = () { run(() { us() + usa() }) }
  h0() + l() + ut() + h1() + w()
  Nil
}

// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `Dep`, which must be given before it gets to the entry point
// src/main.i:22:3: error[E0505]: this type has itself inside it, through a type variable (§4.7)
// 2 errors
// 