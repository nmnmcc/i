(
	testlib = (test) = std/test
	i/core
	i/assertion
)

// P9's boundary arrangement, with a single failing property case.
adapter = (property: () -> Nil ! Assert, caller ! Assert) -> Nil ! Assert {
	report = assertion.catch(property)
	report |> {
		Ok(_) -> Nil
		Error(failure) -> caller.fail(
			AssertionFailure(
				"std/property v1: falsified\nseed: 0\nreason: " <>
					failure.message
			)
		)
	}
}

test("one_property_failure", () { adapter(() { assert(False) }) })

test("nan_before_library", () {
	core.float_from_bits(0x7ff8000000000000)
	Nil
})

test("positive_infinity_before_library", () {
	core.float_from_bits(0x7ff0000000000000)
	Nil
})

test("negative_infinity_before_library", () {
	core.float_from_bits(0xfff0000000000000)
	Nil
})

test("caller_can_catch_final_failure", () {
	result = assertion.catch(() { adapter(() { assert(False) }) })
	result |> {
		Error(_) -> Nil
		Ok(_)    -> assert(False)
	}
})
