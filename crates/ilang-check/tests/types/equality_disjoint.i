// §6.9: `==` and `!=` on two values whose known types are disjoint
// (§4.5 rule 3) give a warning. A member that is `_` or a type
// variable is never disjoint from anything.

// The examples of the spec: warnings.
literals = 1 == "a"

typed = (s: String) { s == 1 }

// The examples of the spec: correct.
with_nil = (x: Int | Nil) { x == Nil }

lists = [1] == []

type_var(T) = (a: T, b: T) { a == b }

type_var_literal(T) = (a: T) { a == 1 }

untyped = (s) { s == 1 }

// `!=` too.
not_equal = 1 != "a"

// Unions that share a member, and unions that do not.
shared = (x: Int | String, y: String | Nil) { x == y }

apart = (x: Int | String, y: Float | Nil) { x != y }

// Tuples of different sizes are different type constructors.
sizes = (1, 2) == (1, 2, 3)

same_size = (1, "a") == (2, 3)

// A function and a value.
function = (f: (Int) -> Int) { f == 1 }

// A part that is `_` is not known, but the rest is.
holes = (x) { (x, 1) == "a" }
