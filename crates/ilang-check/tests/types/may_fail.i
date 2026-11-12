// §7.3 with rules 4 and 10: a binding or a parameter needs `Assert` exactly
// when its pattern may fail for the type of its value.
User {
	User(name: String, age: Int)
}

// Bindings.
pair = (t) {
	(a, b) = t
	a
}

pair_written = (t: (Int, String)) {
	(a, b) = t
	b
}

first = (r) {
	Ok(v) = r
	v
}

annotated = () {
	x: Int = 5
	x
}

literal = (n) {
	1 = n
	n
}

single = (u: User) {
	User(name = name, ..) = u
	name
}

// `Error(Never)` has no values, so `Ok(v)` matches every value.
only_ok = (r: Result(Int, Never)) {
	Ok(v) = r
	v
}

nested = (p: (Result(Int, Never), Bool)) {
	(Ok(x), b) = p
	x
}

from_value = () {
	Ok(v) = Ok(1)
	v
}

list = (xs) {
	[x, ..] = xs
	x
}

list_rest = (xs) {
	[..rest] = xs
	rest
}

prefix = (s) {
	"" <> rest = s
	rest
}

prefix_some = (s) {
	"a" <> rest = s
	rest
}

named = (r) {
	whole = Ok(_) = r
	whole
}

typed_part = (p) {
	(x: Int, y) = p
	y
}

bits = (b) {
	<<x>> = b
	x
}

float = (f) {
	1.0 = f
	f
}

// Parameters.
type_var(A) = (x: A) -> A { x }

param_tuple = ((a, b)) { a }

param_user = (User(name = name, ..)) { name }

param_ctor = (Ok(v)) { v }

param_literal = (0) { 1 }

param_list = ([]) { 1 }
