// §7.5: inside a clause `C(…)` on the subject name `v`, `v` is known to be
// made by `C`. `.f` and a record update read the value through the pattern
// type of the clause (rule 11), so the type of `v` is not changed outside
// the clause: it keeps the values of the other clauses.
T = A | B

A {
	A(f: Int, g: String)
}

B {
	B(h: Int)
}

S {
	S1(k: Int)
}

// `v` is `T | S`; no clause is never used.
field = (v) {
	v |> {
		A(..)  -> v.f
		B(..)  -> 1
		S1(..) -> 0
	}
}

use_field = () { field(S1(1)) }

// With other values at a catch-all.
field_rest = (v) {
	v |> {
		A(..) -> v.f
		_     -> 0
	}
}

use_rest = () { (field_rest("s"), field_rest(B(1))) }

// A record update, with a union for the type of `v`.
update = (v: T | S) {
	v |> {
		A(..) -> A(..v, f: 1)
		_     -> B(0)
	}
}

update_open = (v) {
	v |> {
		A(..) -> A(..v, g: "x")
		_     -> B(0)
	}
}

use_update = () { update_open(1) }

// With a type parameter: the field has the argument of the pattern type.
Box(A) = Full(A) | Empty

Full(A) {
	Full(item: A)
}

Empty {
	Empty
}

item = (b) {
	b |> {
		Full(..) -> b.item
		Empty    -> 0
	}
}

use_item = () { item(Full(2)) + 1 }

// Problems: the field must still be in the constructor of the clause.
wrong = (v) {
	v |> {
		B(..) -> v.f
		_     -> 0
	}
}
