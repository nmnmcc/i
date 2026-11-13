both = () -> Int { pong(2) }

main = () {
	closed()
	Nil
}

run = <A, N>(body: () <- N -> A) { body() }

Dep = (v = Int)

pong = (n: Int) { ping(n) + uset(1) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

loop = (x) { x(x) }

fr = () { run(() { use() }) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

S(N) = (f = () <- N -> Int)

get = () <- (d: Dep) { d.v }

fw = () { use() }
