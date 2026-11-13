Pair(A) = (A, A)
f = (x : Pair("a" | "b")) {
	x |>
		(Pair("a", "a")) { 0 }
		| (Pair("a", "b")) { 1 }
		| (Pair("b", "a")) { 2 }
		| (Pair("b", "b")) { 3 }
}

main = () {
	assert(f(Pair("a", "b")) == 1)
	assert(f(Pair("b", "a")) == 2)
	Nil
}
