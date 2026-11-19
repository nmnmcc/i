// The error contract of `i/fail`: one typed-error API, `Result` as a stored
// outcome, and `Assert` as a separate service. A stop (`panic`) and a `Fail`
// used after its `catch` are covered by `tests/spec/9/fail_stop_not_caught.i`
// and `tests/spec/9/fail_outlived.i`.
(
	testlib = (test) = i/test
	i/core
	i/fail
	i/assertion
	list = i/list
	result = i/result
)

Missing = (key = String)
Denied = (user = String)

lookup = (key : String) -> Int {
	key |>
		("a") { 1 }
		& ("b") { fail.raise(Denied(user = key)) }
		& (_) { fail.raise(Missing(key = key)) }
}

// Handles only `Missing`: catches the union, matches, and raises the rest.
lookup_or_zero = (key : String) -> Int {
	fail.catch(() { lookup(key) }) |>
		(Ok(n)) { n }
		& (Error(_ : Missing)) { 0 }
		& (Error(other)) { fail.raise(other) }
}

int_or_raise = (bad : Bool) -> Int {
	bad |> (True) { fail.raise(1) } & (False) { 5 }
}

deep = (n : Int) -> Int {
	n |> (0) { fail.raise("bottom") } & (_) { deep(n - 1) + 1 }
}

test("raise_reaches_catch_test", () {
	assert(fail.catch(() { fail.raise("no") }) == Error("no"))
	assert(fail.catch(() { deep(3) }) == Error("bottom"))
	assert(fail.catch(() { int_or_raise(True) }) == Error(1))
})

test("catch_ok_passthrough_test", () {
	assert(fail.catch(() { int_or_raise(False) }) == Ok(5))
	assert(fail.catch(() { [1, 2] }) == Ok([1, 2]))
})

test("catch_first_error_test", () {
	r = fail.catch(() {
		list.map([1, 2, 3], (x) { x |> (1) { x } & (_) { fail.raise(x) } })
	})
	assert(r == Error(2))
})

test("from_result_test", () {
	assert(fail.catch(() { fail.from_result(Ok(1)) + 1 }) == Ok(2))
	assert(
		fail.catch(() { fail.from_result(Error("bad")) + 1 }) == Error("bad"),
	)
})

test("map_error_test", () {
	r = fail.catch(() {
		fail.map_error(
			() { int_or_raise(True) },
			(e) { "e" <> core.inspect(e) },
		)
	})
	assert(r == Error("e1"))
	ok = fail.catch(() {
		fail.map_error(
			() { int_or_raise(False) },
			(e) { "e" <> core.inspect(e) },
		)
	})
	assert(ok == Ok(5))
})

test("fail_catch_does_not_take_assert_test", () {
	// `fail.catch` lets the `Assert` failure through to `assertion.catch`.
	outer : Result(Result(Int, String), AssertionFailure) = assertion.catch(() {
		inner : Result(Int, String) = fail.catch(() {
			assert(False)
			1
		})
		inner
	})
	assert(outer |> (Error(_)) { True } & (Ok(_)) { False })
})

test("assertion_catch_does_not_take_fail_test", () {
	r = fail.catch(() { assertion.catch(() { fail.raise("e") }) })
	assert(r == Error("e"))
})

test("narrowed_rethrow_test", () {
	assert(fail.catch(() { lookup_or_zero("a") }) == Ok(1))
	assert(fail.catch(() { lookup_or_zero("x") }) == Ok(0))
	// The outer boundary gets only the case the inner one did not handle.
	denied : Result(Int, Denied) = fail.catch(() { lookup_or_zero("b") })
	assert(denied == Error(Denied(user = "b")))
})

test("stored_outcomes_test", () {
	// Each element is caught on purpose, and the outcomes are kept as data.
	outcomes = list.map(["a", "b", "c"], (key) {
		fail.catch(() { lookup(key) })
	})
	assert(
		outcomes ==
			[Ok(1), Error(Denied(user = "b")), Error(Missing(key = "c"))],
	)
	(values, errors) = result.partition(outcomes)
	assert(values == [1])
	assert(list.length(errors) == 2)
	// A stored outcome can be raised again.
	again = fail.catch(() { fail.from_result(result.all(outcomes)) })
	assert(again == Error(Denied(user = "b")))
})
