// §4.8, names in patterns: `x = q` gives `x` the one member of the known
// type of the value with the type constructor of `q`. A literal or a prefix
// with no such member gives no known type.
lit = (v) {
	v |> {
		x = 1 -> x == "s"
		_     -> False
	}
}

prefix = (v) {
	v |> {
		x = "a" <> r -> x == 1 || r == 1
		_            -> False
	}
}

float = (v) {
	v |> {
		x = 1.0 -> x == "s"
		_       -> False
	}
}

// With a known type, the member is there.
lit_known = (v: Int | String) {
	v |> {
		x = 1 -> x == "s"
		_     -> False
	}
}

prefix_known = (v: String) {
	v |> {
		x = "a" <> r -> r == 1
		_            -> False
	}
}
