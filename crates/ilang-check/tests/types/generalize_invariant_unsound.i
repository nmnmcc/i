// §4.5 rule 4: service arguments must be equal, even for a Sink whose
// ordinary parameter goes in the other direction under rule 6.
Sink(A) {
	Sink(put: (A) -> Nil)
}

needs_int = (s ! Sink(Int)) { s.put(1) }

// Generalization must not turn the required Sink(Int) into Sink(Never).
unsound = () {
	! Sink(Never) = Sink(put: (x: Never) { Nil })
	needs_int()
}

// Ordinary invariant arguments must also remain equal (§4.5 rule 6).
Cell(A) {
	Cell(run: (A) -> A)
}

wrong_cell = (c: Cell(Int)) -> Cell(Never) { c }

// A computation requiring Sink(Int) cannot replace one requiring Sink(Never).
run_never = (body: () -> Nil ! Sink(Never)) { Nil }

bad_computation = () { run_never(needs_int) }
