// Rule 11: matches by places.
// No catch-all: the value must be one the patterns take.
digit = (x) {
	x |> {
		0 -> "zero"
		1 -> "one"
	}
}

// A catch-all at the top place: any value may come.
digit_or = (x) {
	x |> {
		0 -> "zero"
		_ -> "other"
	}
}

// Constructors share their type arguments at one place.
either = (r) {
	r |> {
		Ok(v)    -> v
		Error(e) -> e
	}
}

// A catch-all below a constructor: slack only there.
inner = (r) {
	r |> {
		Ok(1)    -> "one"
		Ok(_)    -> "other"
		Error(e) -> e
	}
}

// Tuples and lists.
swap = (t) {
	t |> {
		(a, b) -> (b, a)
	}
}

sum = (xs) {
	xs |> {
		[]          -> 0
		[x, ..rest] -> x + sum(rest)
	}
}

// A guard is a Bool, and a guarded catch-all gives no slack.
sign = (n) {
	n |> {
		0         -> 0
		m ? m > 0 -> 1
		_         -> -1
	}
}

guarded_only = (x) {
	x |> {
		1         -> 1
		y ? y > 2 -> y
	}
}

// Type tests.
describe = (v) {
	v |> {
		_: Int    -> "int"
		s: String -> s
	}
}

// Choices bind the same names.
choice = (r) {
	r |> {
		Ok(v)    -> v
		Error(v) -> v
	}
}

// The clause expressions share one type.
mixed = (b) {
	b |> {
		True  -> 1
		False -> "no"
	}
}

// Named patterns.
named = (r) {
	r |> {
		whole = Ok(_) -> whole
		Error(_)      -> Ok(0)
	}
}

// A type variable in a type test is a compile error.
bad(A) = (v: A) {
	v |> {
		x: A ? True -> x
		_           -> panic("unreachable")
	}
}

// A catch-all next to a pattern whose parts are tested.
first_one = (p) {
	p |> {
		(1, x) -> x
		_      -> 0
	}
}
