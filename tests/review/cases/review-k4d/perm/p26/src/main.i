S(N) = (f = () <- N -> Int)

Dep = (v = Int)

run = <A, N>(body: () <- N -> A) { body() }

uset: <A, N>(A) <- (Fail(Never), T(A, N), ..N) -> A = (x: A) <- (
	t: T(A, N),
	fail: Fail(Never),
) -> A {
	T(g = g, ..) = t
	g(x)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

get = () <- (d: Dep) { d.v }

both: <N>() <- (Fail(Never), S(N), T(Int, N), ..N) -> Int = () <- (
	fail: Fail(Never),
) -> Int { pong(2) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

loop = (x) { x(x) }

bad: <N>() <- (Fail(Never), ..N) -> Int = () <- (
	s: S(N),
	fail: Fail(Never),
) -> Int {
	S(f = f) = s
	f()
}

main = () {
	closed()
	Nil
}

fw = () { use() }

fr = () { run(() { use() }) }
