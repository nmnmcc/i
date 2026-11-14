// §4.5 rule 4: service arguments must be equal, even for a Sink whose
// ordinary parameter goes in the other direction under rule 6.
Sink(A) = (put = (A) -> Nil)

needs_int : () <- (Sink(Int)) -> Nil = () <- (s : Sink(Int)) { s.put(1) }

// Generalization must not turn the required Sink(Int) into Sink(Never).
unsound = () {
	_syn3_0 : Sink(Never) = Sink(put = (x : Never) { Nil })
	needs_int() <- (_syn3_0)
}

// Ordinary invariant arguments must also remain equal (§4.5 rule 6).
Cell(A) = (run = (A) -> A)

wrong_cell = (c : Cell(Int)) -> Cell(Never) { c }

// A computation requiring Sink(Int) cannot replace one requiring Sink(Never).
run_never = (body : () <- (Sink(Never)) -> Nil) { Nil }

bad_computation = () { run_never(needs_int) }
