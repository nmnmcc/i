S(A) = (A)

T(N) = (f = () <- N -> Nil)

merge = <N, M>(a: T(N), b: T(M)) {
	T(
		f = () {
			a.f()
			b.f()
		},
	)
}

one = () <- (s: S(Int)) { Nil }

two = () <- (s: S(Int)) { Nil }

main = () {
	{
		used = merge(T(f = one), T(f = two))
		used.f()
	} <- (S(1))
}
