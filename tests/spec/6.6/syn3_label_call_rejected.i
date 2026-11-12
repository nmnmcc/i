// spec: 6.6
// expect: compile-error E0507
// syn3-labels: keep

// Syntax three, stage 4: arguments by label are given only to constructors.
// A call of any other function gives its arguments by position.
t9 = (first: Int, second: Int) -> Int { first - second }

main = () {
	t9(5, 1)
	Nil
}
