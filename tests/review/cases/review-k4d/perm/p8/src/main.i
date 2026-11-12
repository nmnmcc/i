fr = () { run(() { use() }) }

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

run = <A, N>(body: () <- N -> A) { body() }

main = () {
	closed()
	Nil
}

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

get = () <- (d: Dep) { d.v }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

S(N) = (f = () <- N -> Int)

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

fw = () { use() }

Dep = (v = Int)

both = () -> Int { pong(2) }
