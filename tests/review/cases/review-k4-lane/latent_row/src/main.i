S(A) {
	S(A)
}

T(N) {
	T(f: () -> Nil ! N)
}

merge(N, M) = (a: T(N), b: T(M)) {
	T(f: () {
		a.f()
		b.f()
	})
}

one = (s ! S(Int)) { Nil }

two = (s ! S(String)) { Nil }


main = () {
	unused = merge(T(f: one), T(f: two))
	Nil
}
