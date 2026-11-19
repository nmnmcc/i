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

two = () <- (s : S(String)) { () }

main = () {
	unused = merge(T(f = one), T(f = two))
	()
}
