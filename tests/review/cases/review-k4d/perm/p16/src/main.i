get = () <- (d: Dep) { d.v }

Dep = (v = Int)

fw = () { use() }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

loop = (x) { x(x) }

run = <A, N>(body: () <- N -> A) { body() }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

pong = (n: Int) { ping(n) + uset(1) }

main = () {
	closed()
	Nil
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

fr = () { run(() { use() }) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }

S(N) = (f = () <- N -> Int)
