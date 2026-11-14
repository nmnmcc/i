Svc(A) = (() -> A)

Row(N) = () <- (Fail(N)) -> Nil

bad = <N>(body : Row(N), one : Svc(Int), two : Svc(Nil)) {
	first : N = one
	second : N = two
	second
}

main = () { Nil }
