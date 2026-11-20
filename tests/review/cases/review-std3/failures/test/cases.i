(
	testlib = (test) = i/test
	i/core
	i/assertion
)

// P9 boundary: retain main's failure sites 12:21, 24:2, 29:2 and 34:2.
adapter = (property : () <- (Assert) -> ()) <- (caller : Assert) -> () {
	report = assertion.catch(property)
	report |>
		(Ok(_)) { () }
		& (Error(failure)) {
			caller.fail(
				AssertionFailure(
					message = "i/property v1: falsified\nseed: 0\nreason: " <>
						failure.message,
				),
			)
		}
}

test("one_property_failure", () { adapter(() { assert(False) }) })

test("nan_before_library", () {
	core.float_from_bits(0x7ff8000000000000)
	()
})

test("positive_infinity_before_library", () {
	core.float_from_bits(0x7ff0000000000000)
	()
})

test("negative_infinity_before_library", () {
	core.float_from_bits(0xfff0000000000000)
	()
})

test("caller_can_catch_final_failure", () {
	result = assertion.catch(() { adapter(() { assert(False) }) })
	result |> (Error(_)) { () } & (Ok(_)) { assert(False) }
})
