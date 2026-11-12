run = <A, N>(body: () <- N -> A) { body() }

fw = () { use() }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

get = () <- (d: Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }

Dep = (v = Int)

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

S(N) = (f = () <- N -> Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

main = () {
	closed()
	Nil
}

fr = () { run(() { use() }) }

both = () -> Int { pong(2) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}
