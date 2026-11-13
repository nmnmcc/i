// spec: 6.6
// expect: compile-error E0507
// syn3-labels: keep

// Syntax three §2.6: positional binders do not give the function label keys.
// Labelled data cannot call this positional shape (spec §6.6).
t9 = (first: Int, second: Int) -> Int { first - second }

main = () {
	t9(second = 1, first = 5)
	Nil
}
