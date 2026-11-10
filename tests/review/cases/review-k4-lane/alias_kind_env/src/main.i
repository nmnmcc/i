Svc(A) {
	Svc(() -> A)
}

Row(N) = () -> Nil ! N

bad(N) = (body: Row(N), one: Svc(Int), two: Svc(Nil)) {
	first: N = one
	second: N = two
	second
}

@main
main = () { Nil }
