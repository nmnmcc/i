// old-syntax-check: E0101
Dep = (v = Int)
get = () <- (d: Dep) { d.v }
Svc(A) = (f = () -> A, m = (A) -> Nil)
ask = <A>() <- (s: Svc(A)) { s.f() }
Cov(A) = (f = () -> A)
askc = <A>() <- (s: Cov(A)) { s.f() }
Cb(N) = (f = () <- N -> Int, m = (N) -> Nil)
call = <N>(c: Cb(N)) {
	Cb(f = f, ..) = c
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
twice = <N>(body: () <- N -> Int) { body() + body() }
with_int = (body) {
	_syn3_0: Svc(Int) = Svc(f = () { 1 }, m = (_: Int) { Nil })
	body() <- (_syn3_0)
}
with_dep = (body) { body() <- (Dep(v = 5)) }
with_cov = (body, v) {
	_syn3_1: Cov(_) = Cov(f = () { v })
	body() <- (_syn3_1)
}
pick = (a, b) { [a, b] }
h0 = () { with_cov(() { askc() }, 3) }
h1 = () {
	x5 = h0() + ask() + run(() { ask() })
	x5 = ask()
	ask()
}

main = () {
	_syn3_2: Cov(Int) = Cov(f = () { 2 })
	{
		with_dep(() { with_int(() { get() + ask() }) })
		Nil
	} <- (_syn3_2)
}
/* LANE
// app/main.get : () -> Int ! Dep
// app/main.ask : () -> A ! Svc(A)
// app/main.askc : () -> A ! Cov(B & A)
// app/main.call : (c: Cb(A)) -> Int ! A
// app/main.run : (body: () -> A ! N) -> A ! N
// app/main.twice : (body: () -> Int ! N) -> Int ! N
// app/main.with_int : (body: () -> A ? E ! N) -> A ? E ! N - Svc(Int)
// app/main.with_dep : (body: () -> A ? E ! N) -> A ? E ! N - Dep
// app/main.with_cov : (body: () -> A ? E ! N, v: B) -> A ? E ! N - Cov(_)
// app/main.pick : (a: A, b: A) -> List(A)
// app/main.h0 : () -> Never
// app/main.h1 : () -> A ! Svc(A & Int)
// app/main.main : () -> Nil
//
// BASE
// app/main.get : () -> Int ! Dep
// app/main.ask : () -> A ! Svc(A)
// app/main.askc : () -> A ! Cov(B & A)
// app/main.call : (c: Cb(A)) -> Int ! A
// app/main.run : (body: () -> A ! N) -> A ! N
// app/main.twice : (body: () -> Int ! N) -> Int ! N
// app/main.with_int : (body: () -> A ? E ! N) -> A ? E ! N - Svc(Int)
// app/main.with_dep : (body: () -> A ? E ! N) -> A ? E ! N - Dep
// app/main.with_cov : (body: () -> A ? E ! N, v: B) -> A ? E ! N - Cov(_)
// app/main.pick : (a: A, b: A) -> List(A)
// app/main.h0 : () -> Never
// app/main.h1 : () -> A ! Svc(A & Int) | Svc(B & Int)
// app/main.main : () -> Nil
*/
