// Comments in the middle of expressions, patterns, and types stay where
// they were, and what comes after them goes on one level in.
f = (x) {
	a = x.field
	// before a field
	b = 2
	t = x.0.1
	// before an index
	m = -x
	// after a minus
	n = (y) -> Int {
		// after an arrow
		y
	}
	c = x + 1
	// before an operator
	p = xs |> g |> h
	// before a pipe
	// before another pipe
	q = x |>
		(A) {
			// before an arrow
			1
		}
		| (B ? c) {
			// before a guard
			2
		}
	d = 2
	// before an equals sign
	z = 1
	//   spaces at the end
	z
}

T = (Int) <- (Fail(String)) -> Int
// after an equals sign in a type
// after a question mark
g = (x) {
	q = x |>
		(A | B) {
			// after a bar
			1
		}
		| (_) { 0 }
	r = call(a = 1)
	// after a label
	s = { xs |> g } == ys
	// after a pipeline
	t = x.f
	// one
	// two
	v = { 1 }.f
	// after an int
	v
}
