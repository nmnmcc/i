// §7.2: in choices `p1 | p2`, a name gets the union of its types.
// §4.8 rule 11: patterns at one place with one type constructor share one
// set of type arguments.
either = (r: Result(Int, String)) { r |> (Ok(v)) { v } | (Error(v)) { v } }

small = (n) { n |> (1 | 2 | 3) { "small" } | (_) { "big" } }

mixed = (v: Int | String | Nil) {
	v |> (1 | _: String) { "one or text" } | (_) { "other" }
}

tuple_choice = (p: (Int, Int)) { p |> ((0, x) | (x, 0)) { x } | (_) { 1 } }

// The choices of one clause are at the same places as the other clauses.
nested_choice = (r) { r |> (Ok(Ok(x)) | Ok(Error(x))) { x } | (Error(e)) { e } }

// Shared type arguments: the patterns at one place with one type
// constructor use one set of new type variables (rule 11), so the two
// `Ok`s give one `Result(b1, b2)`, and `b1` must be `Int | String`. (These
// clauses do not cover every value.)
shared = (r) { r |> (Ok(1)) { 0 } | (Ok("a")) { 1 } | (Error(e)) { e } }

shared_list = (xs) { xs |> ([1]) { 0 } | (["a", ..]) { 1 } | ([]) { 2 } }

shared_tuple = (p) { p |> ((1, True)) { 0 } | (("a", False)) { 1 } }
