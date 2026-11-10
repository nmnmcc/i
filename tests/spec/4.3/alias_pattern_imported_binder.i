// spec: 3, 4.3, 4.7
// expect: compile-error E0537
Box(A) {
	Box(A)
}

value: Box(Int) = Box(1)

Box(Int) = value
