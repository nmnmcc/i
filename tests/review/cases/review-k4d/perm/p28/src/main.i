Dep = (v = Int)

loop = (x) { x(x) }

fr = () { run(() { use() }) }

both = () -> Int { pong(2) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

pong = (n: Int) { ping(n) + uset(1) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

bad = <N>() <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

closed = () {
	both() <- (
		Dep(v = 1),
		T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil }),
		S(f = () { get() }),
	)
}

run = <A, N>(body: () <- N -> A) { body() }

get = () <- (d: Dep) { d.v }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

fw = () { use() }

S(N) = (f = () <- N -> Int)

main = () {
	closed()
	Nil
}

use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}
