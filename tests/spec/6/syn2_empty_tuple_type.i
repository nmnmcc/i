// spec: 3, 6.4
// expect: pass
// Syntax three stage 6: `()` is the empty data type.
(
	main
)
Unit = { () }

main = () {
	u : Unit = { () }
	assert(u == ())
	Nil
}
