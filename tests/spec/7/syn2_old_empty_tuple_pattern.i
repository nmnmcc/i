// spec: 7
// expect: pass
// Syntax three (docs/design/syntax-3.md §2, "Unit"): `()` is the empty
// data, a value, a type and a pattern, so `(())` is a parameter list whose
// one item is the pattern `()`; it is no longer an error.
(
	main
)
f = (()) { 1 }

main = () {
	assert(f(()) == 1)
	Nil
}
