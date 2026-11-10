S(A) {
	S(A)
}

T(N) {
	T(f: () -> Nil ! N)
}

merge(N, M) = (a: T(N), b: T(M)) {
	T(() {
		a.f()
		b.f()
	})
}

one = (s ! S(Int)) { Nil }

two = (s ! S(Int)) { Nil }

@main
main = () {
	! S(Int) = S(1)
	used = merge(T(one), T(two))
	used.f()
}
