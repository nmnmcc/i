run = <A, N>(body: () <- N -> A) { body() }

loop = (x) { x(x) }

fr = () { run(() { use() }) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

S(N) = (f = () <- N -> Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

Dep = (v = Int)

pong = (n: Int) { ping(n) + uset(1) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }

get = () <- (d: Dep) { d.v }

fw = () { use() }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

main = () {
	closed()
	Nil
}
