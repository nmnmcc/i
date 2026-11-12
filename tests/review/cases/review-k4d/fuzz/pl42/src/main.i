Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }
ask(A) = (s ! Svc(A)) { s.f() }
Cov(A) { Cov(f: () -> A) }
askc(A) = (s ! Cov(A)) { s.f() }
Cb(N) { Cb(f: () -> Int ! N, m: (N) -> Nil) }
call(N) = (c: Cb(N)) {
  Cb(f = f, ..) = c
  f()
}
run(A, N) = (body: () -> A ! N) { body() }
twice(N) = (body: () -> Int ! N) { body() + body() }
with_int = (body) {
  ! Svc(Int) = Svc(f: () { 1 }, m: (_: Int) { Nil })
  body()
}
with_dep = (body) {
  ! Dep = Dep(v: 5)
  body()
}
with_cov = (body, v) {
  ! Cov(_) = Cov(f: () { v })
  body()
}
pick = (a, b) { [a, b] }
h0 = () { twice(() { get() }) }
h1 = () {
  p7 = pick(() { get() + ask() }, () { with_dep(() { with_int(() { get() + ask() }) }) + call(Cb(f: () { get() }, m: (_: Dep) { Nil })) })
  l4 = () { h1() + with_int(() { run(() { ask() + 1 }) }) }
  askc()
}

main = () {
  ! Svc(Int) = Svc(f: () { twice(() { get() }) }, m: (_: Int) { Nil })
  ! Svc(Int) = Svc(f: () { with_dep(() { get() }) + ask() + run(() { ask() }) }, m: (_: Int) { Nil })
  ! Cov(Int) = Cov(f: () { 2 })
  ! Svc(Int) = Svc(f: () { get() }, m: (_: Int) { Nil })
  ! Dep = Dep(v: 1)
  with_int(() { run(() { ask() + 1 }) }) + askc()
  Nil
}
