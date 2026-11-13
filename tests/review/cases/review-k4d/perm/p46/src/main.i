uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

run = <A, N>(body: () <- N -> A) { body() }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

both = () -> Int { pong(2) }

main = () {
	closed()
	Nil
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

S(N) = (f = () <- N -> Int)

pong = (n: Int) { ping(n) + uset(1) }

fr = () { run(() { use() }) }

loop = (x) { x(x) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

Dep = (v = Int)

fw = () { use() }

get = () <- (d: Dep) { d.v }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }
