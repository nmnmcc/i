// Constraints that simplification and generalization must keep.

// §4.5 rule 4 and §9.2: an environment provider must be a subtype of
// the needed service, also when the function is generalized.
Reader(S) = (get = () -> S)

with_int = (body) {
	_syn3_0: Reader(Int) = Reader(get = () -> Int { 1 })
	body() <- (_syn3_0)
}

get_s = () <- (r: Reader(String)) { r.get() }

g = () { with_int(() { get_s() }) }

k = () -> String { with_int(() { get_s() }) }

// §4.4: bounds that a variable has through other variables are put
// together too: invariant arguments must be equal, and function types
// must have the same number of parameters.
Cell(A) = (get = () -> A, set = (A) -> Nil)

use_int = (c: Cell(Int)) { c.get() + 1 }

use_str = (c: Cell(String)) { c.get() <> "!" }

pass = (y) { use_str(y) }

through_var = (x) {
	{ (y) { use_str(y) } }(x)
	use_int(x)
}

through_top = (x) {
	pass(x)
	use_int(x)
}

arity = (k) {
	{ (g) { g(1) } }(k)
	k(1, 2)
}

// A list of function types whose environments have two forms.
State(S) = (get = () -> S)

use_state_int = () <- (st: State(Int)) { 1 }

use_state_str = () <- (st: State(String)) { 1 }

nested = [
	Cell(get = () { () { use_state_int() } }, set = (f) { Nil }),
	Cell(get = () { () { use_state_str() } }, set = (f) { Nil }),
]

// Rule 11: a type test takes every value of its type constructor.
Box(A) = (value = A)

unbox = (b: Box(Int)) { b.value }

two_tests = (v: Box(Int | String)) {
	v |> (x: Box(Int)) { unbox(x) + 1 } | (y: Box(String)) { 0 }
}

test_then_ctor = (v: Box(Int | String)) {
	v |> (x: Box(Int)) { unbox(x) + 1 } | (Box(..)) { 0 }
}

lists = (l: List(Int | String)) { l |> (x: List(Int)) { x } | ([]) { [] } }

test_ok = (v: Box(Int) | String) {
	v |> (x: Box(Int)) { unbox(x) } | (s: String) { 0 }
}

// §4.5 rule 2: `a & T <: R` when `a <: R`.
take = <A>(h: () -> A & Int) { h }

p = () { panic("") }

z: Never = take(p)()

not_string: String = take(() { 1 })()
