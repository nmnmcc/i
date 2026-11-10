// §4.5 rule 6: a appears in an input and an output, so it does not change.
Cell(a) {
	Cell(run: (a) -> a)
}

// §4.7: both inferred and written variables at invariant places matter.
inferred = () { Cell((x) { x }) }

written = () -> Cell(a) { Cell((x) { x }) }

int_cell = () -> Cell(Int) { inferred() }

string_cell = () -> Cell(String) { inferred() }

written_int = () -> Cell(Int) { written() }

written_string = () -> Cell(String) { written() }

// Invariance also matters in an input place and inside a covariant type.
ignore = (x: Cell(a)) { Nil }

ignore_inferred = (Cell(run)) { Nil }

ignored = () { (ignore(int_cell()), ignore(string_cell())) }

nested = () { [Cell((x) { x })] }

nested_int = () -> List(Cell(Int)) { nested() }

nested_string = () -> List(Cell(String)) { nested() }

// §4.7 and §4.8 rule 14: local function bindings are generalized too.
local = () {
	new = () { Cell((x) { x }) }
	int: Cell(Int) = new()
	string: Cell(String) = new()
	(int, string)
}
