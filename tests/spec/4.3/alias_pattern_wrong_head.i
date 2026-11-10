// spec: 3, 4.3, 4.7
// expect: compile-error E0535
Box(A) {
	Box(A)
}

value: Int = 1

Box(_) = value
