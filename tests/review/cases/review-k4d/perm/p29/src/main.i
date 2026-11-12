loop = (x) { x(x) }

S(N) = (f = () <- N -> Int)

main = () {
	closed()
	Nil
}

fr = () { run(() { use() }) }

fw = () { use() }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

Dep = (v = Int)

both = () -> Int { pong(2) }

get = () <- (d: Dep) { d.v }

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

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

run = <A, N>(body: () <- N -> A) { body() }

pong = (n: Int) { ping(n) + uset(1) }
