S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}

wrap : <N>(() <- (Fail(Never), S(N), ..N) -> Int) <- (
	Fail(Never),
	..N,
) -> Int = (body : () <- (Fail(Never), S(N), ..N) -> Int) {
	_syn3_0 : S(N) = S(f = () { 1 }, m = (_ : N) { Nil })
	body() <- (_syn3_0)
}

main = () { assert(wrap(() { use() }) == 1) }
