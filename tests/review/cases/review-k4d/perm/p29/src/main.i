loop = (x) { x(x) }

S(N) = (f = () <- N -> Int)

main = () {
	closed()
	Nil
}

fr = () { run(() { use() }) }

fw = () { use() }

bad : <N>() <- (Fail(Never), ..N) -> Int = () <- (
	s : S(N),
	fail : Fail(Never),
) -> Int {
	S(f = f) = s
	f()
}

use = <N>() <- (s : S(N)) {
	S(f = f) = s
	f()
}

ping = (n : Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

Dep = (v = Int)

both : <N>() <- (Fail(Never), S(N), T(Int, N), ..N) -> Int = () <- (
	fail : Fail(Never),
) -> Int { pong(2) }

get = () <- (d : Dep) { d.v }

closed = () {
	_syn3_0 : T(Int, Dep) = T(
		g = (x : Int) { x + get() },
		m = (_ : Dep) { Nil },
	)
	_syn3_1 : S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

uset : <A, N>(A) <- (Fail(Never), T(A, N), ..N) -> A = (x : A) <- (
	t : T(A, N),
	fail : Fail(Never),
) -> A {
	T(g = g, ..) = t
	g(x)
}

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

run = <A, N>(body : () <- N -> A) { body() }

pong = (n : Int) { ping(n) + uset(1) }
