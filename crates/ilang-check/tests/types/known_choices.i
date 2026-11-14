// §7.5 with choices: `x` is known to be made by `C` only when every choice
// binds `x` through `x = C(…)`. A choice that binds `x` as a catch-all, as
// `x: T`, or as `x = _` may give it a value made by another constructor, so
// every `.f` and record update below is a compile error (§6.7).
A = (f = Int)

B = (g = String)
T = A | B
catch_all = (v : T) {
	v |> (A(..) = x | x ? True) { x.f } | (_) { panic("unreachable") }
}

named_discard = (v : T) {
	v |>
		(A(..) = x | _ = x ? True) { A(..x, f = 3) }
		| (_) { panic("unreachable") }
}

first_catch_all = (v : T) {
	v |> (x | A(..) = x ? True) { x.f } | (_) { panic("unreachable") }
}

type_test = (v : T) {
	v |> (A(..) = x | x : T ? True) { x.f } | (_) { panic("unreachable") }
}

nested = (r : Result(T, T)) {
	r |> (Ok(A(..) = x) | Error(x)) { x.f } | (Ok(_)) { 0 }
}

in_tuple = (p : (T, Int)) {
	p |> ((A(..) = x, 1) | (x, _) ? True) { x.f } | (_) { panic("unreachable") }
}

// The subject name, with a choice that does not have `A` at its top.
subject = (v : T) {
	v |> (A(..) | _ ? True) { v.f } | (_) { panic("unreachable") }
}

// Every choice binds `x` through `x = A(…)`: known.
both = (r : Result(T, T)) {
	r |>
		(Ok(A(..) = x) | Error(A(..) = x)) { A(..x, f = x.f + 1) }
		| (_) { B(g = "b") }
}

P = (Int, f = Int)
Q = (g = String)
U = P | Q

both_subject = (v : U) { v |> (P(1, ..) | P(2, ..)) { v.f } | (_) { 0 } }
