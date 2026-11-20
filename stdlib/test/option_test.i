(
	testlib = (test) = i/test
	i/core
	i/fail
	option = (Some, None) = i/option
)

test("all_some_test", () {
	assert(option.all([Some(1), Some(2), Some(3)]) == Some([1, 2, 3]))
	()
})

test("all_empty_test", () {
	assert(option.all([]) == Some([]))
	()
})

test("all_with_none_test", () {
	assert(option.all([Some(1), None(), Some(3)]) == None())
	()
})

test("is_some_with_some_test", () {
	assert(option.is_some(Some(1)))
	()
})

test("is_some_with_none_test", () {
	assert(
		core.not(option.is_some(None())),
	)
	()
})

test("is_none_with_some_test", () {
	assert(
		core.not(option.is_none(Some(1))),
	)
	()
})

test("is_none_with_none_test", () {
	assert(option.is_none(None()))
	()
})

test("to_result_some_test", () {
	assert(option.to_result(Some(1), "possible_error") == Ok(1))
	()
})

test("to_result_none_test", () {
	assert(
		option.to_result(None(), "possible_error") == Error("possible_error"),
	)
	()
})

test("from_result_ok_test", () {
	assert(option.from_result(Ok(1)) == Some(1))
	()
})

test("from_result_error_test", () {
	assert(option.from_result(Error("some_error")) == None())
	()
})

test("unwrap_some_test", () {
	assert(option.unwrap(Some(1), 0) == 1)
	()
})

test("unwrap_none_test", () {
	assert(option.unwrap(None(), 0) == 0)
	()
})

test("lazy_unwrap_some_test", () {
	assert(option.lazy_unwrap(Some(1), () { 0 }) == 1)
	()
})

test("lazy_unwrap_none_test", () {
	assert(option.lazy_unwrap(None(), () { 0 }) == 0)
	()
})

test("map_some_int_test", () {
	assert(option.map(Some(1), (x) { x + 1 }) == Some(2))
	()
})

test("map_some_type_change_test", () {
	assert(option.map(Some(1), (_) { "2" }) == Some("2"))
	()
})

test("map_none_test", () {
	assert(option.map(None(), (x) { x + 1 }) == None())
	()
})

test("flatten_some_some_test", () {
	assert(option.flatten(Some(Some(1))) == Some(1))
	()
})

test("flatten_some_none_test", () {
	assert(option.flatten(Some(None())) == None())
	()
})

test("flatten_none_test", () {
	assert(option.flatten(None()) == None())
	()
})

test("then_some_to_some_test", () {
	assert(option.then(Some(1), (x) { Some(x + 1) }) == Some(2))
	()
})

test("then_some_type_change_test", () {
	assert(option.then(Some(1), (_) { Some("2") }) == Some("2"))
	()
})

test("then_none_test", () {
	assert(option.then(None(), (x) { Some(x + 1) }) == None())
	()
})

test("or_some_some_test", () {
	assert(option.or(Some(1), Some(2)) == Some(1))
	()
})

test("or_some_none_test", () {
	assert(option.or(Some(1), None()) == Some(1))
	()
})

test("or_none_some_test", () {
	assert(option.or(None(), Some(2)) == Some(2))
	()
})

test("or_none_none_test", () {
	assert(option.or(None(), None()) == None())
	()
})

test("lazy_or_some_some_test", () {
	assert(option.lazy_or(Some(1), () { Some(2) }) == Some(1))
	()
})

test("lazy_or_some_none_test", () {
	assert(option.lazy_or(Some(1), () { None() }) == Some(1))
	()
})

test("lazy_or_none_some_test", () {
	assert(option.lazy_or(None(), () { Some(2) }) == Some(2))
	()
})

test("lazy_or_none_none_test", () {
	assert(option.lazy_or(None(), () { None() }) == None())
	()
})

test("values_test", () {
	assert(option.values([Some(1), None(), Some(3)]) == [1, 3])
	()
})

test("values_more_test", () {
	assert(option.values([Some(1), None(), Some(3)]) == [1, 3])
	assert(option.values([]) == [])
	assert(option.values([None()]) == [])
})

test("filter_test", () {
	assert(option.filter(Some(2), int_is_even) == Some(2))
	assert(option.filter(Some(3), int_is_even) == None())
	assert(option.filter(None(), int_is_even) == None())
})

int_is_even = (x : Int) { x % 2 == 0 }

test("map2_test", () {
	assert(option.map2(Some(1), Some(2), (a, b) { a + b }) == Some(3))
	assert(option.map2(Some(1), None(), (a, b) { a + b }) == None())
	assert(option.map2(None(), Some(2), (a : Int, b : Int) { a + b }) == None())
})

test("labels_test", () {
	assert(option.unwrap(None(), 5) == 5)
	assert(option.map(Some(1), (x) { x * 10 }) == Some(10))
	assert(option.lazy_unwrap(None(), () { 7 }) == 7)
})

test("all_long_test", () {
	xs = build(1_000_000, [])
	assert(option.all(xs) == Some(build_plain(1_000_000, [])))
})

build = (n, acc) { n |> (0) { acc } & (_) { build(n - 1, [Some(n), ..acc]) } }

build_plain = (n, acc) {
	n |> (0) { acc } & (_) { build_plain(n - 1, [n, ..acc]) }
}

test("known_types_test", () {
	user = Some(User(name = "ann", age = 30))
	// The field `name` is known from the type of `option.map`.
	assert(option.map(user, (u) { u.name }) == Some("ann"))
	assert(
		option.then(user, (u) {
			{ u.age > 18 } |> (True()) { Some(u.age) } & (False()) { None() }
		}) ==
			Some(30),
	)
})

User = (name = String, age = Int)

test("effects_test", () {
	r = fail.catch(() { option.map(Some(1), (_) { fail.raise("boom") }) })
	assert(r == Error("boom"))
	assert(
		fail.catch(() { option.map(None(), (_) { fail.raise("boom") }) }) ==
			Ok(None()),
	)
})
