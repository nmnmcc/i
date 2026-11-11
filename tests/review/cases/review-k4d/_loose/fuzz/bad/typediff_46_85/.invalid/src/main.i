// old-syntax-check: E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101,E0101
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
  ! Dep = Dep(1)
  x6 = 1 + call(Cb(() { ask() }, (_) { Nil }))
  run(() { ask() })
}

main = () {
  p6 = pick(() { with_int(() { run(() { ask() + 1 }) }) + call(Cb(() { get() }, (_: Dep) { Nil })) + 1 }, () { with_int(() { ask() }) })
  ! Dep = Dep(1)
  ! Cov(Int) = Cov(() { 2 })
  ! Dep = Dep(1)
  askc() + 1 + askc()
  Nil
}

/* LANE
app/main.get : () -> Int ! Dep
app/main.ask : () -> A ! Svc(A)
app/main.askc : () -> A ! Cov(B & A)
app/main.call : (c: Cb(A)) -> Int ! A
app/main.run : (body: () -> A ! N) -> A ! N
app/main.twice : (body: () -> Int ! N) -> Int ! N
app/main.with_int : (body: () -> A ? E ! N) -> A ? E ! N - Svc(Int)
app/main.with_dep : (body: () -> A ? E ! N) -> A ? E ! N - Dep
app/main.with_cov : (body: () -> A ? E ! N, v: B) -> A ? E ! N - Cov(_)
app/main.pick : (a: A, b: A) -> List(A)
app/main.h0 : () -> Never
app/main.h1 : () -> A ! Svc(B & Int & A)
app/main.main : () -> Nil

BASE
app/main.get : () -> Int ! Dep
app/main.ask : () -> A ! Svc(A)
app/main.askc : () -> A ! Cov(B & A)
app/main.call : (c: Cb(A)) -> Int ! A
app/main.run : (body: () -> A ! N) -> A ! N
app/main.twice : (body: () -> Int ! N) -> Int ! N
app/main.with_int : (body: () -> A ? E ! N) -> A ? E ! N - Svc(Int)
app/main.with_dep : (body: () -> A ? E ! N) -> A ? E ! N - Dep
app/main.with_cov : (body: () -> A ? E ! N, v: B) -> A ? E ! N - Cov(_)
app/main.pick : (a: A, b: A) -> List(A)
app/main.h0 : () -> Never
app/main.h1 : () -> A ! Svc(B & Int) | Svc(C & A)
app/main.main : () -> Nil
*/