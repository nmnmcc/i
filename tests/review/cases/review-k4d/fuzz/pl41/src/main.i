Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }
ask(A) = (s ! Svc(A)) { s.f() }
Cov(A) { Cov(f: () -> A) }
askc(A) = (s ! Cov(A)) { s.f() }
Cb(N) { Cb(f: () -> Int ! N, m: (N) -> Nil) }
call(N) = (c: Cb(N)) {
  Cb(f, ..) = c
  f()
}
run(A, N) = (body: () -> A ! N) { body() }
twice(N) = (body: () -> Int ! N) { body() + body() }
with_int = (body) {
  ! Svc(Int) = Svc(() { 1 }, (_: Int) { Nil })
  body()
}
with_dep = (body) {
  ! Dep = Dep(5)
  body()
}
with_cov = (body, v) {
  ! Cov(_) = Cov(() { v })
  body()
}
pick = (a, b) { [a, b] }
h0 = () { twice(() { get() }) }
h1 = () {
  ! Svc(Int) = Svc(() { call(Cb(() { ask() }, (_) { Nil })) + with_dep(() { with_int(() { get() + ask() }) }) }, (_: Int) { Nil })
  with_int(() { ask() })
}

main = () {
  ! Cov(Int) = Cov(() { with_cov(() { askc() }, 3) })
  ! Cov(Int) = Cov(() { twice(() { get() }) })
  ! Cov(Int) = Cov(() { 2 })
  ! Svc(Int) = Svc(() { get() }, (_: Int) { Nil })
  ! Svc(String) = Svc(() { "s" }, (_: String) { Nil })
  get() + 1 + with_int(() { ask() })
  Nil
}
