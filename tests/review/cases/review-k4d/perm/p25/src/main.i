both = () -> Int { pong(2) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)

loop = (x) { x(x) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

run = <A, N>(body: () <- N -> A) { body() }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

pong = (n: Int) { ping(n) + uset(1) }

main = () {
	closed()
	Nil
}

fr = () { run(() { use() }) }

get = () <- (d: Dep) { d.v }

Dep = (v = Int)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

fw = () { use() }
