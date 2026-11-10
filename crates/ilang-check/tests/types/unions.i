// §4.4: members with one type constructor are put together.
State(S) {
	State(get: () -> S, set: (S) -> Nil)
}

use_int = (st ! State(Int)) { 1 }

use_string = (st ! State(String)) { 1 }

// An environment with two forms of an invariant type is a compile error.
both = () {
	use_int()
	use_string()
}

ok = () {
	use_int()
	use_int()
}

// Functions are one head.
fns = [(x: Int) { x }, (y: String) { 1 }]

// Tuples of one size are one head; of other sizes, not.
tuples = [(1, "a"), ("b", 2)]

sizes = [(1, 2), (1, 2, 3)]

// A difference that cannot be: `List(Int | String) - List(String)`.
bad_diff = (x: List(Int | String) - List(String)) { x }

// The right side of `-` may not have a type variable.
var_diff(A) = (x: Int - A) { x }
