// §7.4: the clauses of a match must cover every value of the type of the
// subject. A missing pattern is shown.
Circle = (radius = Float)

Square = (side = Float)
Point = ()
Shape = Circle | Square | Point
Flags = (admin = Bool, active = Bool)

covered = (s: Shape) {
	s |> (Circle(..)) { 1 } | (Square(..)) { 2 } | (Point) { 3 }
}

missing_ctor = (s: Shape) { s |> (Circle(..)) { 1 } | (Square(..)) { 2 } }

missing_nested = (r: Result(Bool, Nil)) {
	r |> (Ok(True)) { 1 } | (Error(_)) { 2 }
}

missing_member = (v: Int | String) {
	v |> (_: Int ? True) { 1 } | (_: String) { 2 }
}

missing_label = (f: Flags) {
	f |> (Flags(True = admin, ..)) { 1 } | (Flags(True = active, ..)) { 2 }
}

// Without annotations, the type comes from the patterns.
inferred = (r) { r |> (Ok(1)) { 1 } | (Error(_)) { 2 } }

// A guard is not counted.
guarded = (b: Bool) { b |> (True) { 1 } | (False ? True) { 2 } }

// Literal patterns never cover their type.
ints = (n) { n |> (0) { 0 } | (1) { 1 } }

floats = (x) { x |> (0.0) { 0 } | (_ ? False) { panic("unreachable") } }

strings = (s) { s |> ("a") { 0 } | ("b") { 1 } }

prefixes = (s) { s |> ("a" <> r) { r } | ("b") { "b" } }

// `"" <> r` matches every string.
all_strings = (s) { s |> ("a" <> r) { r } | ("" <> r) { r } }

bit_arrays = (b) { b |> (<<x>>) { x } | (<<>>) { 0 } }

all_bits = (b) {
	b |> (<<rest:bits>>) { 1 } | (_ ? False) { panic("unreachable") }
}

// Lists: the empty list, and a cons of an element and the rest.
no_empty = (xs) { xs |> ([x, ..]) { x } | (_ ? False) { panic("unreachable") } }

no_long = (xs) { xs |> ([]) { 0 } | ([x]) { x } }

lists = (xs) { xs |> ([]) { 0 } | ([x]) { x } | ([x, y, ..]) { x + y } }

list_items = (xs: List(Bool)) { xs |> ([]) { 0 } | ([True, ..]) { 1 } }

// Tuples.
tuples = (p: (Bool, Bool)) { p |> ((True, _)) { 1 } | ((_, True)) { 2 } }

// Types with no values are covered.
only_ok = (r: Result(Int, Never)) { r |> (Ok(x)) { x } }

empty = [] |> ([]) { 0 }

// The type test makes `a` an `a & Int`, which it covers.
any = <A>(x: A) { x |> (_: Int) { 1 } }

// A type that comes from a function that is not known.
unknown = (f) { f() |> (True) { 1 } | (False) { 2 } }

// The values of the subject are those that come to it: `Error` has none.
no_error = () { Ok(1) |> (Ok(x)) { x } }
