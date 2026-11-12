get = () <- (d: Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

main = () {
	closed()
	Nil
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

S(N) = (f = () <- N -> Int)

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

Dep = (v = Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

loop = (x) { x(x) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

fr = () { run(() { use() }) }

both = () -> Int { pong(2) }

run = <A, N>(body: () <- N -> A) { body() }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}
