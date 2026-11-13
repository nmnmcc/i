Pair(A) = (A, A)
f = (x) {
	x |>
		(Pair("a", "a")) { 0 }
		| (Pair("a", "b")) { 1 }
		| (Pair("b", "a")) { 2 }
		| (Pair("b", "b")) { 3 }
}
