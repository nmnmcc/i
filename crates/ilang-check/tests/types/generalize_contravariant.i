// §4.5 rule 6: A goes in the other direction in Sink(A).
Sink(A) {
	Sink(put: (A) -> Nil)
}

// §4.7: a variable only in input places has no effect on the meaning.
new = () { Sink((_) { Nil }) }

int_sink = () -> Sink(Int) { new() }

string_sink = () -> Sink(String) { new() }

// The input of make reverses the direction of Sink's parameter.
Back(A) {
	Back(sink: Sink(A))
}

make(A) = (s: Sink(A)) { Back(s) }

// The shared variable must remain when it relates an input to an output.
wrap = (f) { Sink(f) }
