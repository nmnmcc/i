// spec: 4.3
// expect: compile-error E0202
// A member of a sum is a labelled type of its own (syntax three §2.8): a
// sum whose members are not defined types is rejected, as a definition with
// two constructors was before the conversion.
(
	main
)
Shape = Circle(Float) | Square(Float)

main = () {
	assert(True)
	Nil
}
