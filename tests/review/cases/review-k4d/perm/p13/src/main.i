closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }

main = () {
	closed()
	Nil
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

fw = () { use() }

run = <A, N>(body: () <- N -> A) { body() }

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

get = () <- (d: Dep) { d.v }

fr = () { run(() { use() }) }

S(N) = (f = () <- N -> Int)

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

Dep = (v = Int)

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)
