pong = (n: Int) { ping(n) + uset(1) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

run = <A, N>(body: () <- N -> A) { body() }

main = () {
	closed()
	Nil
}

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

Dep = (v = Int)

fr = () { run(() { use() }) }

get = () <- (d: Dep) { d.v }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

fw = () { use() }

loop = (x) { x(x) }

S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}
