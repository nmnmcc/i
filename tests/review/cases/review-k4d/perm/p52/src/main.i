closed = () {
	_syn3_0 : T(Int, Dep) = T(
		g = (x : Int) { x + get() },
		m = (_ : Dep) { Nil },
	)
	_syn3_1 : S(Dep) = S(f = () { get() })
	both() <- (Dep(v = 1), _syn3_0, _syn3_1)
}

pong = (n : Int) { ping(n) + uset(1) }

T(A, N) = (g = (A) <- N -> A, m = (N) -> Nil)

uset : <A, N>(A) <- (Fail(Never), T(A, N), ..N) -> A = (x : A) <- (
	t : T(A, N),
	fail : Fail(Never),
) -> A {
	T(g = g, ..) = t
	g(x)
}

main = () {
	closed()
	Nil
}

fr = () { run(() { use() }) }

Dep = (v = Int)

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

S(N) = (f = () <- N -> Int)

ping = (n : Int) { n |> (0) { use() } | (_) { pong(n - 1) } }

fw = () { use() }

run = <A, N>(body : () <- N -> A) { body() }

both : <N>() <- (Fail(Never), S(N), T(Int, N), ..N) -> Int = () <- (
	fail : Fail(Never),
) -> Int { pong(2) }

loop = (x) { x(x) }

get = () <- (d : Dep) { d.v }
