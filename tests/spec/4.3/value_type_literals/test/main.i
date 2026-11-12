(
	testlib = (test) = std/test
	model/api
)

// Joint §4: the completed literal fields survive a package boundary.
Remote = api.config

same: Remote = api.Config("GET", 8080)

test("value_annotation", () {
	method = "GET"
	same: method = "GET"
	assert(same == "GET")
})

test("literal_pattern_and_union", () {
	number: api.Number = 2
	assert(number == 2)
})

test("literal_fields_from_a_value", () {
	value: api.ConfigType = api.Config("GET", 8080)
	assert(value.method == "GET")
	assert(value.port == 8080)
	assert(same == value)
})

accepts = (s: Int | String) {
	s |> {
		_: api.Methods -> True
		_              -> False
	}
}

test("literal_alias_runtime_tests", () {
	assert(accepts("GET"))
	assert(accepts("POST"))
	assert(accepts("DELETE") == False)
	assert(accepts(1) == False)
	assert(accepts(100000000000000000000) == False)
})

Box(A) {
	Box(value: A)
}

test("inferred_access_is_distinct_from_known_facts", () {
	copy = (s: String) {
		x = "a" = s
		p = (Box(x), 0).0.value
		q: p = "b"
		q
	}
	assert(copy("a") == "b")
})

test("narrowed_binder_and_original_subject", () {
	read = (s: String) {
		s |> {
			_: String - "a" -> 0
			key -> {
				same: key = "a"
				original: s = "b"
				assert(same == "a")
				assert(original == "b")
				1
			}
		}
	}
	assert(read("a") == 1)
	assert(read("b") == 0)
})
