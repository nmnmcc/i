ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

Dep = (v = Int)

fr = () { run(() { use() }) }

run = <A, N>(body: () <- N -> A) { body() }

pong = (n: Int) { ping(n) + uset(1) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

main = () {
	closed()
	Nil
}

fw = () { use() }

loop = (x) { x(x) }

get = () <- (d: Dep) { d.v }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }
