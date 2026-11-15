run = <A, N>(body : () <- N -> A) { body() }

fw = () { use() }

pong = (n : Int) { ping(n) + uset(1) }

both : <N>() <- (Fail(Never), S(N), T(Int, N), ..N) -> Int = () <- (
	fail : Fail(Never),
) -> Int { pong(2) }

uset : <A, N>(A) <- (Fail(Never), T(A, N), ..N) -> A = (x : A) <- (
	t : T(A, N),
	fail : Fail(Never),
) -> A {
	T(g = g, ..) = t
	g(x)
}

loop = (x) { x(x) }

closed = () {
	_syn3_0 : T(Int, Dep) = T(
		g = (x : Int) { x + get() },
		m = (_ : Dep) { Nil },
	)
	_syn3_1 : S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

use = <N>() <- (s : S(N)) {
	S(f = f) = s
	f()
}

bad : <N>() <- (Fail(Never), ..N) -> Int = () <- (
	s : S(N),
	fail : Fail(Never),
) -> Int {
	S(f = f) = s
	f()
}

get = () <- (d : Dep) { d.v }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

main = () {
	closed()
	Nil
}

Dep = (v = Int)

S(N) = (f = () <- N -> Int)

ping = (n : Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

fr = () { run(() { use() }) }
