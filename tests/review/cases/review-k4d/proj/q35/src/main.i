S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) -> Int { s.f() }
fw: <N>() <- (Fail(Never), S(N), ..N) -> Int = () <- (
	fail: Fail(Never),
) -> Int { use() }
