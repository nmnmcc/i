Svc(A) = (() -> A)

Row(N) = () <- N -> ()

bad = <N>(body : Row(N), one : Svc(Int), two : Svc(())) {
	first : N = one
	second : N = two
	second
}

main = () { () }
