use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}

both = () -> Int { pong(2) }

Dep = (v = Int)

closed = () {
	_syn3_0: T(Int, Dep) = T(g = (x: Int) { x + get() }, m = (_: Dep) { Nil })
	_syn3_1: S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

run = <A, N>(body: () <- N -> A) { body() }

fw = () { use() }

fr = () { run(() { use() }) }

uset = <A, N>(x: A) <- (t: T(A, N)) -> A {
	T(g = g, ..) = t
	g(x)
}

S(N) = (f = () <- N -> Int)

loop = (x) { x(x) }

main = () {
	closed()
	Nil
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

bad: <N>() <- (Fail(Never), ..N) -> Int = () <- (s: S(N)) -> Int {
	S(f = f) = s
	f()
}

pong = (n: Int) { ping(n) + uset(1) }

ping = (n: Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

get = () <- (d: Dep) { d.v }
