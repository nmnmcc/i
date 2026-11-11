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
h0 = () { run(h0) }
h1 = () {
  ! Cov(Int) = Cov(() { with_dep(() { with_int(() { get() + ask() }) }) + askc() })
  ! Svc(String) = Svc(() { "s" }, (_: String) { Nil })
  with_int(() { ask() })
}

main = () {
  ! Cov(Int) = Cov(() { with_int(() { run(() { ask() + 1 }) }) + 1 + askc() })
  ! Dep = Dep(1)
  ! Svc(Int) = Svc(() { get() }, (_: Int) { Nil })
  call(Cb(() { ask() }, (_) { Nil })) + twice(h1) + with_dep(() { get() })
  Nil
}
