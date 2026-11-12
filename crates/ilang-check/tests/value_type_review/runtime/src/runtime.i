(
	alias_tests
	fresh_instances
	inferred_not_known
	schema_library
)

port: Int = 1

Port = port

Pair(A, B) = (A, B)

parts: (Int, String) = (1, "s")

Pair(Number, Text) = parts

id = (x) { x }

match_alias = (v: Int | String) {
	v |> {
		n: Port -> n + 1
		_       -> 0
	}
}

alias_tests = () {
	assert(match_alias(2) == 3)
	assert(match_alias("s") == 0)
}

fresh_instances = () {
	a: id = (x: Int) { x + 1 }
	b: id = (x: String) { x <> "!" }
	assert(a(2) == 3)
	assert(b("s") == "s!")
}

inferred_not_known = () {
	copy = (s: String) {
		x = "a" = s
		y: x = "b"
		y
	}
	assert(copy("a") == "b")
}

Schema(D, E) {
	Schema(decode: (String) -> D, encode: (E) -> String)
}

make(D, E) = (d: (String) -> D, e: (E) -> String) { Schema(decode: d, encode: e) }

user = make(
	(s: String) -> (String, Int) { (s, 1) }
	(v: (String, Int)) -> String { v.0 }
)

Schema(User, _) = user

schema_library = () {
	v: User = user.decode("ok")
	assert(v == ("ok", 1))
	assert(user.encode(("yes", 2)) == "yes")
}
