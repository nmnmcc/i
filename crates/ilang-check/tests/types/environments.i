// §4.5 rule 4 and §9.2: environments are compared head by head, with equal
// arguments, whatever the variance of the service type.
(
	i/control
)

Yield(A) = (emit = (A) -> Nil)

Source(A) = (next = () -> A)

collect: <A, N>(() <- (Yield(A), ..N) -> Nil) <- N -> List(A) = (body) {
	control.boundary((b) {
		_syn3_0: Yield(A) = Yield(
			emit = (x) {
				control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
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
	collect(() <- (y: Yield(Int)) {
		y.emit(1)
		y.emit(2)
	})
}

// Without an annotation, the argument comes from the service type.
inferred = <A>() { collect(() <- (y: Yield(A)) { y.emit("s") }) }

// A function that needs `Yield(Int)` may not be given `Yield(Never)`.
run_never = (body: () <- (Yield(Never)) -> Nil) { 1 }

bad_never = () { run_never(() <- (y: Yield(Int)) { y.emit(1) }) }

// Equal arguments are fine.
run_int = (body: () <- (Yield(Int)) -> Nil) { 1 }

good_int = () { run_int(() <- (y: Yield(Int)) { y.emit(1) }) }

// Merging: two forms of one type constructor in an environment are a
// compile error, also for a contravariant or a covariant service.
emit_int = () <- (y: Yield(Int)) { y.emit(1) }

emit_string = () <- (y: Yield(String)) { y.emit("a") }

both_yields = () {
	emit_int()
	emit_string()
}

next_int = () <- (s: Source(Int)) { s.next() + 1 }

next_string = () <- (s: Source(String)) { s.next() <> "" }

both_sources = () {
	next_int()
	next_string()
}

// In the environment of a function type made from two of them.
both_fns = [() { emit_int() }, () { emit_string() }]

// In a written environment.
written = () <- (y: Yield(Int), z: Source(Int)) -> Nil { y.emit(z.next()) }

// Rule 9: the member that an environment statement takes out must be equal
// to the service it gives.
emit_any = () <- (y: Yield(Int | String)) { y.emit(1) }

given_narrow = () {
	_syn3_1: Yield(Int) = Yield(emit = (x) { Nil })
	emit_any() <- (_syn3_1)
}

given_same = () {
	_syn3_2: Yield(Int | String) = Yield(emit = (x) { Nil })
	emit_any() <- (_syn3_2)
}
