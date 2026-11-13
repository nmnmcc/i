S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

main = () {
	closed()
	Nil
}

loop = (x) { x(x) }

run = <A, N>(body: () <- N -> A) { body() }

Dep = (v = Int)

pong = (n: Int) { ping(n) + uset(1) }

get = () <- (d: Dep) { d.v }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

fw = () { use() }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

both = () -> Int { pong(2) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

fr = () { run(() { use() }) }
