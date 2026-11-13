bad: <N>() <- (Fail(Never), ..N) -> Int = () <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

main = () {
	closed()
	Nil
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

get = () <- (d: Dep) { d.v }

Dep = (v = Int)

fr = () { run(() { use() }) }

loop = (x) { x(x) }

both = () -> Int { pong(2) }

fw = () { use() }

pong = (n: Int) { ping(n) + uset(1) }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

run = <A, N>(body: () <- N -> A) { body() }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)
