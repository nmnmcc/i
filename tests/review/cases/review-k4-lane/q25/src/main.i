S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) { s.f() }

fw: <N>() <- (Fail(Never), S(N), ..N) -> Int = () <- (
	fail: Fail(Never),
) -> Int { use() }

main = () {
	_syn3_0: S(Never) = S(f = () { 1 })
	assert(fw() == 1) <- (_syn3_0)
}
