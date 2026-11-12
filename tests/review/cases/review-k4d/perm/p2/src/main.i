closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

S(N) = (f = () <- N -> Int)

pong = (n: Int) { ping(n) + uset(1) }

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

fw = () { use() }

both = () -> Int { pong(2) }

Dep = (v = Int)

loop = (x) { x(x) }

fr = () { run(() { use() }) }

main = () {
	closed()
	Nil
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

get = () <- (d: Dep) { d.v }

run = <A, N>(body: () <- N -> A) { body() }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}
