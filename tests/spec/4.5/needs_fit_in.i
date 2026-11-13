// spec: 4.5, 9.2
// expect: pass

// Rule 4: the caller's Yield(Int | String) is a value subtype of the
// Yield(Int) that the function needs.
Yield(A) {
	Yield(emit: (A) -> Nil)
}

Narrow = () -> Nil ! Yield(Int)

Wide = () -> Nil ! Yield(Int | String)

fit = (f: Narrow) -> Wide { f }

need_int = (y ! Yield(Int)) { y.emit(1) }

main = () {
	! Yield(Int | String) = Yield((x: Int | String) { Nil })
	fit(need_int)()
}
