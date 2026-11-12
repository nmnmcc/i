// §1.7: `a` and then `- b` on the next line are two statements.
two = {
	a
	-b
}

// `-` at the end of the line: one expression.
one = { a - b }

// `(` on the same line is a call; first on a line it begins a statement.
calls = {
	f(x)
	f
	(x)
}

// Lines that begin with `|>`, `.`, or a binary operator go on.
goes_on = {
	x |> f |> g
	y.field.0
	a * b + c == d
}

// A break inside `( )`, `[ ]`, `<< >>` is a `,`.
list = [1, 2, 3]
tuple = (a, b)
call = f(a, -b)
bits = <<1, 2:u8>>

// Statements and clauses may share a line.
shared = {
	x = 1
	y = 2
	x + y
}
clauses = b |> (True) { 1 } | (False) { 0 }

// `|`, `if`, and `->` at the start of a line go on.
case_lines = x |> (1 | 2 ? y) { 3 } | (_) { 4 }

// `?` at the start of a line goes on; `{` of the body on the same line.
typed = (x) -> Int { x }

// A break ends a clause: `-1` begins the next one.
negative = n |> (0) { 1 } | (-1) { 2 } | (_) { 3 }

// Attributes are part of the item after them.
@custom
main = () { Nil }
