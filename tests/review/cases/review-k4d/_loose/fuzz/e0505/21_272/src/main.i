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
  ! S(Dep) = S(() { get() }, (_: Dep) { Nil })
  a = h1()
  ! Dep = Dep(3)
  w = () {
    ! T(T(S(Never))) = T(() { h0() }, (_: T(S(Never))) { Nil })
    1
  }
  a + ut() + run(() { ut() }) + w()
  Nil
}

// src/main.i:24:1: error[E0505]: this type has itself inside it, through a type variable (§4.7)
// src/main.i:24:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `T(a)`, which must be given before it gets to the entry point
// src/main.i:24:1: error[E0522]: `main` is an entry point, so its environment may only have host services and `Assert` (§10.2), but it needs `a`, which must be given before it gets to the entry point
// src/main.i:29:39: error[E0501]: `Dep` is not a subtype of `T(S(Never))` (the type at 29:24 needs it)
// 4 errors
// 