get = () <- (d: Dep) { d.v }

fw = () { use() }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

Dep = (v = Int)

both = () -> Int { pong(2) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

main = () {
	closed()
	Nil
}

S(N) = (f = () <- N -> Int)

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

run = <A, N>(body: () <- N -> A) { body() }

fr = () { run(() { use() }) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }
