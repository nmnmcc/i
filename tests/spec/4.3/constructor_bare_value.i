// spec: 4.3, 6.3, 6.7
// expect: compile-error E0501

// `True` alone is the constructor, of type `() -> True`, not a value of Bool.
x : Bool = True()

main = () {
	assert(x)
}
