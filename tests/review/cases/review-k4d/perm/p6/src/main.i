loop = (x) { x(x) }

main = () {
	closed()
	Nil
}

get = () <- (d: Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

both = () -> Int { pong(2) }

Dep = (v = Int)

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

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

S(N) = (f = () <- N -> Int)

run = <A, N>(body: () <- N -> A) { body() }

fr = () { run(() { use() }) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)
