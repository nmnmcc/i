// §4.5 rules 6 and 7: variance from the definitions.
Box(A) {
	Box(value: A)
}

Sink(A) {
	Sink(put: (A) -> Nil)
}

Cell(A) {
	Cell(get: () -> A, set: (A) -> Nil)
}

Wrap(A) {
	Wrap(inner: Box(A))
}

Back(A) {
	Back(sink: Sink(Sink(A)))
}

// Covariant: two forms are put together by union.
boxes = [Box(value: 1), Box(value: "a")]

// Contravariant: by intersection.
sinks = (s1: Sink(Int), s2: Sink(String)) { [s1, s2] }

// Through other definitions.
wraps = [Wrap(inner: Box(value: 1)), Wrap(inner: Box(value: "a"))]

backs = (b1: Back(Int), b2: Back(String)) { [b1, b2] }

// Subtyping by variance.
widen = (b: Box(Int)) -> Box(Int | String) { b }

narrow = (s: Sink(Int | String)) -> Sink(Int) { s }

// Invariant: the forms must be equal.
cells = (c1: Cell(Int), c2: Cell(String)) { [c1, c2] }

cell_ok = (c1: Cell(Int), c2: Cell(Int)) { [c1, c2] }

bad_widen = (c: Cell(Int)) -> Cell(Int | String) { c }

// List is covariant (rule 7).
lists = (a: List(Int)) -> List(Int | String) { a }
