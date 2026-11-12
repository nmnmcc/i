fr = () { run(() { use() }) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

main = () {
	closed()
	Nil
}

get = () <- (d: Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

run = <A, N>(body: () <- N -> A) { body() }

both = () -> Int { pong(2) }

loop = (x) { x(x) }

S(N) = (f = () <- N -> Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

Dep = (v = Int)

fw = () { use() }
