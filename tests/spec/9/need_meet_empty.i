// spec: 4.4, 9.2
// expect: compile-error E0515

// The value path computes the empty argument meet immediately; the need
// path must report that this service cannot be provided.
Read(A) = (value = A)

value_meet = (x : Read(Int) & Read(String)) -> Read(Never) { x }

one = () <- (r : Read(Int)) { Nil }

two = () <- (r : Read(String)) { Nil }

bad = () {
	one()
	two()
}
