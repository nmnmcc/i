// §4.7: the shown type is made simpler, and keeps its meaning.
(
	i/fail
)

// Arguments that do not change.
Ref(A) = (get = () -> A, set = (A) -> Nil)

read = <A>(r: Ref(A)) { r.get() }

either = <A, B>(r1: Ref(A), r2: Ref(B), c: Bool) {
	c |> (True) { r1 } | (False) { r2 }
}

make_int = (x) {
	Ref(
		get = () { x },
		set = (y) {
			y + 1
			Nil
		},
	)
}

make_any = (x) { Ref(get = () { x }, set = (y) { Nil }) }

// Written variables on one side only are taken out.
one_side = <A>(x: A) -> Int { x }

errors_only = () -> Int { fail.raise(1) }

empty_list = () {
	y: List(A) = []
	y
}

// Intersections of unions.
inter_unions = <A>(x: A & { Int | String } & { Int | Nil }) { x }

// Only the type constructors of `H` are shown.
rest = (v) {
	v |>
		([]) { 0 }
		| ([_, ..]) { 1 }
		| (Ok(_)) { 2 }
		| (Error(_)) { 3 }
		| (other) { other }
}

// A binding that is not generalized: each type variable is the union of
// the types that come to it.
id = (x) { x }

mono = id((x) { x })

use_int = () { mono(1) }

use_string = () { mono("a") }

// After a problem, the forms of an environment are shown as they are.
State(S) = (get = () -> S)

state_int = () <- (st: State(Int)) { 1 }

state_str = () <- (st: State(String)) { 1 }

both = () {
	state_int()
	state_str()
}
