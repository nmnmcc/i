loop = (x) { x(x) }

main = () {
	closed()
	Nil
}

get = () <- (d: Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

both = () -> Int { pong(2) }

Dep = (v = Int)

bad: <N>() <- (Fail(Never), ..N) -> Int = () <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

fw = () { use() }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

S(N) = (f = () <- N -> Int)

run = <A, N>(body: () <- N -> A) { body() }

fr = () { run(() { use() }) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)
