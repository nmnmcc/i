Dep = (v = Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

loop = (x) { x(x) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

pong = (n: Int) { ping(n) + uset(1) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

both = () -> Int { pong(2) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

fr = () { run(() { use() }) }

run = <A, N>(body: () <- N -> A) { body() }

fw = () { use() }

S(N) = (f = () <- N -> Int)

get = () <- (d: Dep) { d.v }

main = () {
	closed()
	Nil
}
