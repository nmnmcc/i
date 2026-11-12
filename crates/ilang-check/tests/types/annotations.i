(
	i/fail
)

// §4.7: annotations are used in inference, and a type variable in an
// annotation may become a more exact type.
exact = <A>(x: A) -> A { x + 1 }

general = <A>(x: A) -> A { x }

holes = (x: List(_)) { x }

inner = <A>(x: A) -> A {
	same: (A) -> A = (y) { y }
	same(x)
}

// Two local bindings next to each other have their own `a`.
apart = () {
	f: <A>(A) -> A = <A>(x) { x }
	g: <A>(A) -> A = <A>(x) { x }
	(f(1), g("s"))
}

typed_value: Int = 1

fn_type: (Int) -> Int = (x) { x }

union_arg = (x: Int | String) { x }

inter_arg = <A>(x: A & Int) -> A { x }

list_union: List(Int) | List(String) = [1]

result_written = <A, E>(r: Result(A, E)) -> A {
	r |> (Ok(v)) { v } | (Error(e)) { fail.raise(e) }
}

diff_written = (x: Int | String | Nil) -> { Int | String | Nil } - String { 1 }

alias_use = (p: Point) { p }

Point = { (Int, Int) }

Pair(A) = { (A, A) }

pair_use = (p: Pair(String)) { p }

// The inferred type must agree with an annotation.
wrong: String = 1

wrong_ret = (x) -> String { x + 1 }
