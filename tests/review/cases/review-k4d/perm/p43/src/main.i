both = () -> Int { pong(2) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

Dep = (v = Int)

run = <A, N>(body: () <- N -> A) { body() }

fr = () { run(() { use() }) }

fw = () { use() }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

loop = (x) { x(x) }

get = () <- (d: Dep) { d.v }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

main = () {
	closed()
	Nil
}
