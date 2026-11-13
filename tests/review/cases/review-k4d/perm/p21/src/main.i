both = () -> Int { pong(2) }

main = () {
	closed()
	Nil
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

get = () <- (d: Dep) { d.v }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

run = <A, N>(body: () <- N -> A) { body() }

Dep = (v = Int)

fr = () { run(() { use() }) }

S(N) = (f = () <- N -> Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

loop = (x) { x(x) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}
