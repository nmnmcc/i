// §4.8: known types. Most lines compare a value with `==` to a value that
// cannot be equal to it (§6.9): the error shows the known type of each
// side. No error means that a side has no known type.
(
	i/core
)

User {
	User(name: String, age: Int)
}

Box(A) {
	Box(value: A)
}

// Literals; a bit array; a tuple, with `_` for a part with no known type;
// a list, with `List(_)` when a part has no known type; `[]`.
lit_int = () { 1 == "a" }

lit_float = () { 1.0 == 1 }

lit_bits = () { <<1>> == 1 }

lit_tuple = (x) { (1, x) == 1 }

lit_list = () { [1, "a"] == 1 }

lit_list_unknown = (x) { [x, 1] == 1 }

lit_empty = () { [] == 1 }

// Operators have their result types; `a |> f(…)` is the call.
op_int = (a, b) { a + b == "s" }

op_bool = (a, b) { { a < b } == 1 }

op_string = (a, b) { a <> b == 1 }

op_neg = (a) { -.a == 1 }

op_pipe = () { 1 |> core.to_float == 1 }

// A a match has the union of its clause expressions, when they all have one.
case_union = (b: Bool) {
	{
		b |> {
			True  -> 1
			False -> "a"
		}
	} == 1.0
}

case_unknown = (b: Bool, x) {
	{
		b |> {
			True  -> 1
			False -> x
		}
	} == "a"
}

// A record update is the call of its constructor.
update = (b: Box(Int)) { Box(..b, value: "a") == 1 }

update_rest = (u: User) { User(..u, age: 2) == 1 }

// Parameters: `x: T` gives `T`; no type gives none; `x ! T` gives `T`.
param_typed = (s: String) { s == 1 }

param_untyped = (s) { s == 1 }

param_env = (u ! User) { u == 1 }

// Names with a type scheme, simplified: built-in, imported, a module member,
// and a generalized binding outside its own value and group.
id = (x) { x }

scheme_builtin = () { panic == 1 }

scheme_member = () { core.not == 1 }

scheme_top = () { id == 1 }

scheme_local = () {
	g = (x: Int) { x }
	g == 1
}

// A constructor has its function type, with labels.
ctor_fn = () { Box == 1 }

ctor_value = () { Nil == 1 }

// A function expression: its labels, the written types or `_`, and the
// written `->` part or else the known type of its body.
fn_written = () { (x: Int) -> String { "a" } == 1 }

fn_body = () { (x: Int, y) { x } == 1 }

// Inside its own value, and in its group, a name has only what is written.
own = (n: Int) {
	n |> {
		0 -> 0
		_ -> {
			own == 1
			0
		}
	}
}

group_a = (n: Int) -> Int {
	group_b == 1
	n
}

group_b = (m) { group_a(m) }

own_local = () {
	f: (Int) -> _ = (n) {
		f == 1
		n
	}
	f
}

// A call: the result type with the types that matching gives.
call = () { id(1) == "a" }

call_unknown = (x) { id(x) == "a" }

call_ctor = () { Box(value: 1) == 1 }

// Matching a union: the type variable gets what the other members do not
// take. `_` matches nothing.
unwrap(A) = (x: A | Nil) -> A {
	x |> {
		Nil -> panic("nil")
		y   -> y
	}
}

call_union = (u: User | Nil) { unwrap(u) == 1 }

call_union_field = (u: User | Nil) { unwrap(u).name }

first(A, B) = (p: (A, B)) -> A { p.0 }

call_hole = (x) { first((x, 1)) == 1 }

// A type variable that gets more than one type gets their union.
either(A) = (x: A, y: A) -> A { x }

call_join = () { either(1, "a") == 1.0 }

// Differences and intersections in a scheme: the types are put in, and the
// result is worked out again (§4.4).
drop_nil = (x) {
	x |> {
		Nil -> panic("nil")
		y   -> y
	}
}

call_diff = (u: User | Nil) { drop_nil(u) == 1 }

inc = (x) {
	x + 1
	x
}

call_inter = () { inc(1) == "a" }

// `.f` and `.k` have the field and part types.
field = (u: User) { u.name == 1 }

field_generic = (b: Box(String)) { b.value == 1 }

part = (t: (Int, String)) { t.1 == 1 }

// A block has the known type of its last expression.
block = () {
	{
		x = 1
		x
	} == "a"
}

// Names in patterns.
pattern_tuple = (p: (Int, String)) {
	(a, b) = p
	b == 1
}

pattern_ctor = (b: Box(String)) {
	Box(value = value) = b
	value == 1
}

pattern_named = (b: Box(String)) {
	whole = Box(..) = b
	whole == 1
}

pattern_annotation = (x) {
	y: Int = x
	y == "a"
}

pattern_list = (xs: List(String)) {
	xs |> {
		[x, ..rest] -> x == 1 || rest == 1
		[]          -> False
	}
}

pattern_case_part = (b: Box(Int | String)) {
	b |> {
		Box(value = value) ? True -> value == 1.0
		_                 -> panic("unreachable")
	}
}

// A catch-all gets `B` less the members that rule 12 takes out.
pattern_narrowed = (x: Int | String | Nil) {
	x |> {
		_: Nil -> 0
		other -> { other == Nil } |> {
			True  -> 1
			False -> 2
		}
	}
}

// A name that is not generalized gets its known type from its value.
not_general = () {
	y = id(1)
	y == "a"
}

// The examples of §4.8: a parameter with no type has no known type, but a
// name in a clause pattern gets the known part type.
Option(A) {
	Some(A)
	None
}

untyped_field = (u) { u.name }

option_name = (opt: Option(User)) {
	opt |> {
		Some(u) -> u.name
		None    -> ""
	}
}

// An annotation label wins over the label of the value.
annotated: (Int) -> Int = (y) { y }

annotated_call = annotated(1)
