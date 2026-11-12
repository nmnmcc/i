(
	testlib = (test) = std/test
	i/core
	i/fail
	list = std/list
	result = std/result
	option = (Some, None) = std/option
)

test("is_ok_with_ok_test", () {
	assert(result.is_ok(Ok(1)))
	Nil
})

test("is_ok_with_error_test", () {
	assert(core.not(result.is_ok(Error(1))))
	Nil
})

test("is_error_with_ok_test", () {
	assert(core.not(result.is_error(Ok(1))))
	Nil
})

test("is_error_with_error_test", () {
	assert(result.is_error(Error(1)))
	Nil
})

test("map_ok_increment_test", () {
	assert(result.map(Ok(1), (x) { x + 1 }) == Ok(2))
	Nil
})

test("map_ok_type_change_test", () {
	assert(result.map(Ok(1), (_) { "2" }) == Ok("2"))
	Nil
})

test("map_error_test", () {
	assert(result.map(Error(1), (x) { x + 1 }) == Error(1))
	Nil
})

test("map_error_ok_test", () {
	assert(result.map_error(Ok(1), (x) { x + 1 }) == Ok(1))
	Nil
})

test("map_error_error_test", () {
	assert(
		result.map_error(Error(1), (x) { ("ok", x + 1) }) == Error(("ok", 2))
	)
	Nil
})

test("flatten_ok_ok_test", () {
	assert(result.flatten(Ok(Ok(1))) == Ok(1))
	Nil
})

test("flatten_ok_error_test", () {
	assert(result.flatten(Ok(Error(1))) == Error(1))
	Nil
})

test("flatten_error_test", () {
	assert(result.flatten(Error(1)) == Error(1))
	Nil
})

test("flatten_error_error_test", () {
	assert(result.flatten(Error(Error(1))) == Error(Error(1)))
	Nil
})

test("try_error_test", () {
	assert(result.then(Error(1), (x) { Ok(x + 1) }) == Error(1))
	Nil
})

test("try_ok_to_ok_test", () {
	assert(result.then(Ok(1), (x) { Ok(x + 1) }) == Ok(2))
	Nil
})

test("try_ok_type_change_test", () {
	assert(result.then(Ok(1), (_) { Ok("type change") }) == Ok("type change"))
	Nil
})

test("try_ok_to_error_test", () {
	assert(result.then(Ok(1), (_) { Error(1) }) == Error(1))
	Nil
})

test("unwrap_ok_test", () {
	assert(result.unwrap(Ok(1), 50) == 1)
	Nil
})

test("unwrap_error_test", () {
	assert(result.unwrap(Error("nope"), 50) == 50)
	Nil
})

test("unwrap_error_error_test", () {
	assert(result.unwrap_error(Error(1), 50) == 1)
	Nil
})

test("unwrap_error_ok_test", () {
	assert(result.unwrap_error(Ok("nope"), 50) == 50)
	Nil
})

test("lazy_unwrap_ok_test", () {
	assert(result.lazy_unwrap(Ok(1), () { 50 }) == 1)
	Nil
})

test("lazy_unwrap_error_test", () {
	assert(result.lazy_unwrap(Error("nope"), () { 50 }) == 50)
	Nil
})

test("or_ok_ok_test", () {
	assert(result.or(Ok(1), Ok(2)) == Ok(1))
	Nil
})

test("or_ok_error_test", () {
	assert(result.or(Ok(1), Error("Error 2")) == Ok(1))
	Nil
})

test("or_error_ok_test", () {
	assert(result.or(Error("Error 1"), Ok(2)) == Ok(2))
	Nil
})

test("or_error_error_test", () {
	assert(result.or(Error("Error 1"), Error("Error 2")) == Error("Error 2"))
	Nil
})

test("lazy_or_ok_ok_test", () {
	assert(result.lazy_or(Ok(1), () { Ok(2) }) == Ok(1))
	Nil
})

test("lazy_or_ok_error_test", () {
	assert(result.lazy_or(Ok(1), () { Error("Error 2") }) == Ok(1))
	Nil
})

test("lazy_or_error_ok_test", () {
	assert(result.lazy_or(Error("Error 1"), () { Ok(2) }) == Ok(2))
	Nil
})

test("lazy_or_error_error_test", () {
	assert(
		result.lazy_or(Error("Error 1"), () { Error("Error 2") }) ==
			Error("Error 2")
	)
	Nil
})

test("all_ok_test", () {
	assert(result.all([Ok(1), Ok(2), Ok(3)]) == Ok([1, 2, 3]))
	Nil
})

test("all_with_error_test", () {
	assert(result.all([Ok(1), Error("a"), Error("b"), Ok(3)]) == Error("a"))
	Nil
})

test("partition_empty_test", () {
	assert(result.partition([]) == ([], []))
	Nil
})

test("partition_all_ok_test", () {
	assert(result.partition([Ok(1), Ok(2), Ok(3)]) == ([1, 2, 3], []))
	Nil
})

test("partition_all_error_test", () {
	assert(
		result.partition([Error("a"), Error("b"), Error("c")]) ==
			([], ["a", "b", "c"])
	)
	Nil
})

test("partition_mixed_test", () {
	assert(
		result.partition([Ok(1), Error("a"), Ok(2), Error("b"), Error("c")]) ==
			([1, 2], ["a", "b", "c"])
	)
	Nil
})

test("partition_tco_ok_test", () {
	// TCO test
	_ = list.repeat(Ok(1), 1_000_000) |> result.partition
	Nil
})

test("partition_tco_error_test", () {
	// TCO test
	_ = list.repeat(Error("a"), 1_000_000) |> result.partition
	Nil
})

test("replace_error_with_error_test", () {
	assert(result.replace_error(Error(Nil), "Invalid") == Error("Invalid"))
	Nil
})

test("replace_error_with_ok_test", () {
	assert(result.replace_error(Ok(Nil), "Invalid") == Ok(Nil))
	Nil
})

test("replace_with_ok_test", () {
	assert(result.replace(Ok(Nil), "OK") == Ok("OK"))
	Nil
})

test("replace_with_error_test", () {
	assert(result.replace(Error(Nil), "Invalid") == Error(Nil))
	Nil
})

test("values_test", () {
	assert(result.values([Ok(1), Error(""), Ok(3)]) == [1, 3])
	Nil
})

test("try_recover_ok_test", () {
	assert(result.recover(Ok(1), (_) { panic("unreachable") }) == Ok(1))
	Nil
})

test("try_recover_error_to_ok_test", () {
	assert(result.recover(Error(1), (n) { Ok(n + 1) }) == Ok(2))
	Nil
})

test("try_recover_error_to_error_test", () {
	assert(
		result.recover(Error(1), (_) { Error("failed to recover") }) ==
			Error("failed to recover")
	)
	Nil
})

test("from_option_test", () {
	assert(result.from_option(Some(1), "none") == Ok(1))
	assert(result.from_option(None, "none") == Error("none"))
})

test("to_option_test", () {
	assert(result.to_option(Ok(1)) == Some(1))
	assert(result.to_option(Error("e")) == None)
})

test("to_error_and_catch_test", () {
	assert(result.catch(() { result.to_error(Ok(1)) + 1 }) == Ok(2))
	assert(
		result.catch(() { result.to_error(Error("bad")) + 1 }) == Error("bad")
	)
	assert(result.catch(() { fail.raise(42) }) == Error(42))
})

test("recover_test", () {
	assert(result.recover(Error(1), (e) { Ok(e * 10) }) == Ok(10))
	assert(result.recover(Ok(5), (e) { Ok(e * 10) }) == Ok(5))
	assert(
		result.recover(Error(1), (e) { Error("still " <> core.inspect(e)) }) ==
			Error("still 1")
	)
})

test("all_long_test", () {
	xs = build(1_000_000, [])
	assert(result.all(xs) == Ok(build_plain(1_000_000, [])))
	assert(result.all([Ok(1), Error("a"), Error("b")]) == Error("a"))
})

test("partition_long_test", () {
	(oks, errors) = result.partition(build(1_000_000, []))
	assert(list.length(oks) == 1_000_000)
	assert(errors == [])
})

build = (n, acc) {
	n |> {
		0 -> acc
		_ -> build(n - 1, [Ok(n), ..acc])
	}
}

build_plain = (n, acc) {
	n |> {
		0 -> acc
		_ -> build_plain(n - 1, [n, ..acc])
	}
}

test("map_effects_test", () {
	r = fail.catch(() { result.map(Ok(1), (_) { fail.raise("boom") }) })
	assert(r == Error("boom"))
})

test("error_union_test", () {
	// `then` keeps the error type of the first result; a union is fine.
	r: Result(Int, String | Int) = result.then(Ok(1), (x) {
		{ x > 0 } |> {
			True  -> Error(0)
			False -> Error("negative")
		}
	})
	assert(r == Error(0))
})
