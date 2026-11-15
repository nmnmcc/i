S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}
wrap : <N>(() <- (Fail(Never), S(N), ..N) -> Int, S(N)) <- (
	Fail(Never),
	..N,
) -> Int = (body : () <- (Fail(Never), S(N), ..N) -> Int, svc : S(N)) {
	_syn3_0 : S(N) = svc
	body() <- (_syn3_0)
}
