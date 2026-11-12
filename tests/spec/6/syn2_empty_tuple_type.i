// spec: 3, 6.4
// expect: pass
// Syntax three (docs/design/syntax-3.md D5): a labelled type whose data is
// `()` is a value. `Unit = ()` is no longer an error; it defines `Unit`.
(
	main
)
Unit = ()

main = () {
	u: Unit = Unit
	assert(u == Unit)
	Nil
}
