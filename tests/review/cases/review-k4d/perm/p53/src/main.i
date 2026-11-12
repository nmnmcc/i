get = () <- (d: Dep) { d.v }

fr = () { run(() { use() }) }

Dep = (v = Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

fw = () { use() }

S(N) = (f = () <- N -> Int)

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

run = <A, N>(body: () <- N -> A) { body() }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

main = () {
	closed()
	Nil
}

both = () -> Int { pong(2) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

loop = (x) { x(x) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

pong = (n: Int) { ping(n) + uset(1) }
