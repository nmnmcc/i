// spec: 4.8
// expect: warning W0505

// Syntax three D4: a fitted constructor pattern whose type is disjoint from
// the known type of the value never matches; the compiler warns.
Ok(A) { Ok(A) }
Error(E) { Error(E) }

_never = () {
	r = Error("no")
	Ok(v) = r
	v
}

main = () { assert(1 == 1) }
