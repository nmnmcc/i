// §4.4: in a written type, a union with two or more type variables only in
// an output place, and an intersection with a type variable only in an
// input place.
union_in(A, B) = (v: A | B) { v }

inter_out(A) = (v: Int) -> A & Int { v }

inter_both(A, B) = (x: A, y: B) -> A & B { x }

inter_param_of_param(A) = (f: (A & Int) -> Int) { 1 }

union_in_alias(A, B) = (v: Either(A, B)) { v }

Either(A, B) = A | B

// Allowed.
inter_in(A) = (x: A & Int) -> A { x }

union_param_of_param(A, B) = (f: (A | B) -> Int) { 1 }

union_out(A, B): (Int) -> A | B = (x) { panic("") }

Wrap(A, B) {
	Wrap(value: A | B)
}
