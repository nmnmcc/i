S(A) = (A)

T(N) = (f = () <- N -> ())

merge = <N, M>(a : T(N), b : T(M)) {
	T(
		f = () {
			a.f()
			b.f()
		},
	)
}

one = () <- (s : S(Int)) { () }

two = () <- (s : S(Int)) { () }

main = () {
	_syn3_0 : S(Int) = S(1)
	{
		used = merge(T(f = one), T(f = two))
		used.f()
	} <- (_syn3_0)
}
