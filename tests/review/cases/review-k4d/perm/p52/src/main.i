closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

main = () {
	closed()
	Nil
}

fr = () { run(() { use() }) }

Dep = (v = Int)

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

S(N) = (f = () <- N -> Int)

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

fw = () { use() }

run = <A, N>(body: () <- N -> A) { body() }

both = () -> Int { pong(2) }

loop = (x) { x(x) }

get = () <- (d: Dep) { d.v }
