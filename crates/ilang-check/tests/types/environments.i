// §4.5 rule 4 and §9.2: needs merge by service direction (R4).
// A provided service must be a subtype of the required service.
(
	i/control
)

Yield(A) = (emit = (A) -> ())

// §4.1: a function-only Source(Never) is inhabited. A required value
// retains this fixture's negative empty-intersection intent under Decision 1.
Source(A) = (value = A, next = () -> A)

collect : <A, N>(() <- (Yield(A), ..N) -> ()) <- N -> List(A) = (body) {
	control.boundary((b) {
		_syn3_0 : Yield(A) = Yield(
			emit = (x) {
				control.suspend(b, (k) { [x, ..control.resume(k, ())] })
			},
		)
		{
			body()
			[]
		} <- (_syn3_0)
	})
}

// The hole of contravariance: `a` must be `Int`, not `Never`.
numbers = () {
	collect(() <- (y : Yield(Int)) {
		y.emit(1)
		y.emit(2)
	})
}

// Without an annotation, the argument comes from the service type.
inferred = <A>() { collect(() <- (y : Yield(A)) { y.emit("s") }) }

// A function that needs `Yield(Int)` may not be given `Yield(Never)`.
run_never = (body : () <- (Yield(Never)) -> ()) { 1 }

bad_never = () { run_never(() <- (y : Yield(Int)) { y.emit(1) }) }

// Equal arguments are fine.
run_int = (body : () <- (Yield(Int)) -> ()) { 1 }

good_int = () { run_int(() <- (y : Yield(Int)) { y.emit(1) }) }

// R4: contravariant Yield merges Int and String into Int | String.
// Covariant Source intersects them, so its incompatible need is an error.
emit_int = () <- (y: Yield(Int)) { y.emit(1) }

emit_string = () <- (y : Yield(String)) { y.emit("a") }

both_yields = () {
	emit_int()
	emit_string()
}

next_int = () <- (s : Source(Int)) { s.next() + 1 }

next_string = () <- (s : Source(String)) { s.next() <> "" }

both_sources = () {
	next_int()
	next_string()
}

// In the environment of a function type made from two of them.
both_fns = [() { emit_int() }, () { emit_string() }]

// In a written environment.
written = () <- (y : Yield(Int), z : Source(Int)) -> () { y.emit(z.next()) }

// Provide: Yield(Int) cannot implement a need for Yield(Int | String);
// Yield(Int | String) can implement that need.
emit_any = () <- (y: Yield(Int | String)) { y.emit(1) }

given_narrow = () {
	_syn3_1 : Yield(Int) = Yield(emit = (x : Int) { () })
	emit_any() <- (_syn3_1)
}

given_same = () {
	_syn3_2 : Yield(Int | String) = Yield(emit = (x : { Int | String }) { () })
	emit_any() <- (_syn3_2)
}
