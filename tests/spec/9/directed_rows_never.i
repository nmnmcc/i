// spec: 4.5, 9.2
// expect: compile-error E0515

// Rule A: a provider `Yield(Never)` does not meet a need `Yield(Int)`.
Yield(A) {
	Yield(emit: (value: A) -> Nil)
}

emit(A) = (x: A, y ! Yield(A)) { y.emit(x) }

f = () {
	! Yield(Never) = Yield((x) { x })
	emit(1)
}
