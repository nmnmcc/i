use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

Dep = (v = Int)

S(N) = (f = () <- N -> Int)

loop = (x) { x(x) }

pong = (n: Int) { ping(n) + uset(1) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

run = <A, N>(body: () <- N -> A) { body() }

main = () {
	closed()
	Nil
}

both = () -> Int { pong(2) }

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

fw = () { use() }

fr = () { run(() { use() }) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

get = () <- (d: Dep) { d.v }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}
