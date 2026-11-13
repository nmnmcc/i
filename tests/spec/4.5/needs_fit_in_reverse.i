// spec: 4.5, 9.2
// expect: compile-error E0515

// Rule 4: a caller providing only Yield(Int) cannot run a function that
// needs Yield(Int | String).
Yield(A) {
	Yield(emit: (A) -> Nil)
}

Narrow = () -> Nil ! Yield(Int)

Wide = () -> Nil ! Yield(Int | String)

bad = (f: Wide) -> Narrow { f }
