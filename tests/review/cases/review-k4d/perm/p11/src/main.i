fw = () { use() }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

get = () <- (d: Dep) { d.v }

loop = (x) { x(x) }

main = () {
	closed()
	Nil
}

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

Dep = (v = Int)

S(N) = (f = () <- N -> Int)

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

both = () -> Int { pong(2) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

fr = () { run(() { use() }) }

run = <A, N>(body: () <- N -> A) { body() }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}
