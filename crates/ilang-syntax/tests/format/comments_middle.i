// Comments in the middle of expressions, patterns, and types stay where
// they were, and what comes after them goes on one level in.

f = (x) {
	a = x // before a field
		.field
	b = 2
	t = x.0 // before an index
		.1
	m = - // after a minus
		x
	n = (y) -> // after an arrow
		Int { y }
	c = x // before an operator
		+ 1
	p = xs // before a pipe
		|> g // before another pipe
		|> h
	q =  x |> {
		A // before an arrow
			-> 1
		B // before a guard
			? c -> 2
	}
	d // before an equals sign
		= 2
	z = //   spaces at the end   
		1
	z
}

T = // after an equals sign in a type
	(Int) -> Int ^ // after a question mark
	String

g = (x) {
	q =  x |> {
		A | // after a bar
			B -> 1
		_ -> 0
	}
	r = call(a: // after a label
		1)
	s = xs |> g == // after a pipeline
		ys
	t = x // one
		// two
		.f
	v = 1 // after an int
		.f
	v
}
