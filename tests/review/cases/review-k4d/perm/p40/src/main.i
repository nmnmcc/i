get = () <- (d: Dep) { d.v }

loop = (x) { x(x) }

fr = () { run(() { use() }) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

run = <A, N>(body: () <- N -> A) { body() }

Dep = (v = Int)

pong = (n: Int) { ping(n) + uset(1) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)

fw = () { use() }

both = () -> Int { pong(2) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

main = () {
	closed()
	Nil
}

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}
