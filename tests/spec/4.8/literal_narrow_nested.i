// spec: 4.8, 7.4
// expect: pass

(
	testlib = (test) = std/test
)

g = (s: String - "a") -> Int { 7 }

h = (s: "a") -> Int { 8 }

scalar = (s: String) {
	s |> {
		"a" -> 0
		t   -> g(t)
	}
}

choices = (s: String) {
	s |> {
		"a" | "b" -> 0
		t         -> g(t)
	}
}

nested = (r: Result(String, Nil)) {
	r |> {
		Ok("a")  -> 0
		Ok(rest) -> g(rest)
		Error(_) -> 1
	}
}

tuple = (p: (String, Bool)) {
	p |> {
		("a", _) -> 0
		(t, _)   -> g(t)
	}
}

list = (xs: List(String)) {
	xs |> {
		["a", ..] -> 0
		[t, ..]   -> g(t)
		[]        -> 2
	}
}

except = (s: String) {
	s |> {
		x: String - "a" -> 0
		t               -> h(t)
	}
}

finite = (m: "a" | "b") {
	m |> {
		"a" -> 1
		"b" -> 2
	}
}

prefix = (m: "ab" | "b") {
	m |> {
		"a" <> _ -> 1
		"b"      -> 2
	}
}

non_empty = (s: String) {
	s |> {
		"" -> 0
		"a" <> rest -> { rest == "a" } |> {
			True  -> 1
			False -> 2
		}
		_ -> 3
	}
}

prefix_except = (p: (String, Int)) {
	p |> {
		(x: String - "ab", _) -> 0
		("a" <> r, t)         -> t
		_                     -> 2
	}
}

test("narrowing_runs", () {
	assert(scalar("b") == 7)
	assert(choices("c") == 7)
	assert(nested(Ok("b")) == 7)
	assert(nested(Error(Nil)) == 1)
	assert(tuple(("b", False)) == 7)
	assert(list(["b", "a"]) == 7)
	assert(except("a") == 8)
	assert(finite("a") == 1)
	assert(finite("b") == 2)
	assert(prefix("ab") == 1)
	assert(prefix("b") == 2)
	assert(non_empty("aa") == 1)
	assert(prefix_except(("ab", 5)) == 5)
})
