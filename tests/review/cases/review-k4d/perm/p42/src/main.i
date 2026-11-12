main = () {
	closed()
	Nil
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

S(N) = (f = () <- N -> Int)

both = () -> Int { pong(2) }

run = <A, N>(body: () <- N -> A) { body() }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

fw = () { use() }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

fr = () { run(() { use() }) }

loop = (x) { x(x) }

Dep = (v = Int)

pong = (n: Int) { ping(n) + uset(1) }

get = () <- (d: Dep) { d.v }
