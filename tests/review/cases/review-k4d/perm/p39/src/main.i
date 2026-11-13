T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

loop = (x) { x(x) }

fw = () { use() }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

both = () -> Int { pong(2) }

fr = () { run(() { use() }) }

pong = (n: Int) { ping(n) + uset(1) }

Dep = (v = Int)

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

run = <A, N>(body: () <- N -> A) { body() }

main = () {
	closed()
	Nil
}

get = () <- (d: Dep) { d.v }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)
