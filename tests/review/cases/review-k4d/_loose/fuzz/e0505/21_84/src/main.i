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
h0 = () { run(() { us() }) + 1 }
h1 = () { usa() }

main = () {
  ! Dep = Dep(v: 1)
  w = () {
    ! T(Dep) = T(f: () { get() }, m: (_: Dep) { Nil })
    run(() { run(() { usa() }) }) + us()
  }
  ! S(Dep | T(Dep)) = S(f: () { ut() + get() }, m: (_: Dep | T(Dep)) { Nil })
  l = () { run(() { us() + usa() }) }
  h0() + l() + ut() + h1() + w()
  Nil
}

// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `T(Dep)`, which must be given before it gets to the entry point
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `T(a & Dep)`, which must be given before it gets to the entry point
// src/main.i:21:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `a`, which must be given before it gets to the entry point
// src/main.i:23:3: error[E0505]: this type has itself inside it, through a type variable (§4.7)
// 4 errors
// 