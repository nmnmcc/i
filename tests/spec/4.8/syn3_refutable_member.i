// spec: 4.8, 7.3
// expect: stop "assertion failed: pattern Some(q) did not match None"

// Syntax three D4: fitting the constructor pattern `Some(q)` to a union of
// one-constructor types needs `B <: Q | r - Q`, not `B <: Q`. The binding
// may fail, so it reports to `Assert`, as with a multi-constructor type.
Some(A) = (A)
None = ()
Option(A) = Some(A) | None

both = (b: Bool) -> Option(Int) { b |> (True) { Some(1) } | (False) { None } }

main = () {
	Some(p) = both(True)
	assert(p == 1)
	Some(q) = both(False)
	assert(q == 2)
}
