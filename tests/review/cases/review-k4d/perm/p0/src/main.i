Dep = (v = Int)

get = () <- (d: Dep) { d.v }

S(N) = (f = () <- N -> Int)

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

run = <A, N>(body: () <- N -> A) { body() }

fw = () { use() }

fr = () { run(() { use() }) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

pong = (n: Int) { ping(n) + uset(1) }

both = () -> Int { pong(2) }

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

loop = (x) { x(x) }

main = () {
	closed()
	Nil
}
