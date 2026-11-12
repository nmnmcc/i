bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

run = <A, N>(body: () <- N -> A) { body() }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

pong = (n: Int) { ping(n) + uset(1) }

Dep = (v = Int)

S(N) = (f = () <- N -> Int)

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

fr = () { run(() { use() }) }

loop = (x) { x(x) }

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

get = () <- (d: Dep) { d.v }

fw = () { use() }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

both = () -> Int { pong(2) }
