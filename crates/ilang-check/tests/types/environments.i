// §4.5 rule 4 and §9.2: environments are compared head by head, with equal
// arguments, whatever the variance of the service type.
(
	i/control
)

Yield(A) {
	Yield(emit: (A) -> Nil)
}

Source(A) {
	Source(next: () -> A)
}

collect(A, N) = (body: () -> Nil ! Yield(A) | N) -> List(A) ! N {
	control.boundary((b) {
		! Yield(A) = Yield(emit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		body()
		[]
	})
}

// The hole of contravariance: `a` must be `Int`, not `Never`.
numbers = () {
	collect((y ! Yield(Int)) {
		y.emit(1)
		y.emit(2)
	})
}

// Without an annotation, the argument comes from the service type.
inferred(A) = () { collect((y ! Yield(A)) { y.emit("s") }) }

// A function that needs `Yield(Int)` may not be given `Yield(Never)`.
run_never = (body: () -> Nil ! Yield(Never)) { 1 }

bad_never = () { run_never((y ! Yield(Int)) { y.emit(1) }) }

// Equal arguments are fine.
run_int = (body: () -> Nil ! Yield(Int)) { 1 }

good_int = () { run_int((y ! Yield(Int)) { y.emit(1) }) }

// Merging: two forms of one type constructor in an environment are a
// compile error, also for a contravariant or a covariant service.
emit_int = (y ! Yield(Int)) { y.emit(1) }

emit_string = (y ! Yield(String)) { y.emit("a") }

both_yields = () {
	emit_int()
	emit_string()
}

next_int = (s ! Source(Int)) { s.next() + 1 }

next_string = (s ! Source(String)) { s.next() <> "" }

both_sources = () {
	next_int()
	next_string()
}

// In the environment of a function type made from two of them.
both_fns = [() { emit_int() }, () { emit_string() }]

// In a written environment.
written = (y ! Yield(Int), z ! Source(Int)) -> Nil ! Yield(Int) | Source(Int) {
	y.emit(z.next())
}

// Rule 9: the member that an environment statement takes out must be equal
// to the service it gives.
emit_any = (y ! Yield(Int | String)) { y.emit(1) }

given_narrow = () {
	! Yield(Int) = Yield(emit: (x) { Nil })
	emit_any()
}

given_same = () {
	! Yield(Int | String) = Yield(emit: (x) { Nil })
	emit_any()
}
