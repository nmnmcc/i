T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

loop = (x) { x(x) }

fw = () { use() }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

bad: <N>() <- (Fail(Never), ..N) -> Int = () <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

fr = () { run(() { use() }) }

get = () <- (d: Dep) { d.v }

Dep = (v = Int)

main = () {
	closed()
	Nil
}

run = <A, N>(body: () <- N -> A) { body() }

S(N) = (f = () <- N -> Int)

pong = (n: Int) { ping(n) + uset(1) }
