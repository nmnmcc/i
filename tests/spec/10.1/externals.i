// spec: 10.1, B.2, B.4
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

// External functions that the host gives (`ilang_host::services::testing`).
@external("test:math/v1", "add")
add: (Int, Int) -> Int

@external("test:math/v1", "hypot")
hypot: (Float, Float) -> Float

// A top-level value may call an external function: imports may be called
// while the values are worked out (B.1).
seven = add(3, 4)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

apply = (f: (Int, Int) -> Int, a: Int, b: Int) { f(a, b) }

test("calls_are_made_as_written", () {
	check(
		core.inspect(add(40, 2)),
		"42",
	)
	check(
		core.inspect(add(-5, 2)),
		"-3",
	)
	check(
		core.inspect(add(0, 0)),
		"0",
	)
	// Big numbers go through buffers (B.2, B.3).
	check(
		core.inspect(add(100000000000000000000000000000, 1)),
		"100000000000000000000000000001",
	)
	check(
		core.inspect(add(-4294967296, 4294967295)),
		"-1",
	)
	check(
		core.inspect(hypot(3.0, 4.0)),
		"5.0",
	)
	check(core.inspect(seven), "7")
})

test("an_external_function_is_a_value", () {
	check(
		core.inspect(apply(add, 1, 2)),
		"3",
	)
	f = add
	check(
		core.inspect(f(b = 1, a = 2)),
		"3",
	)
})
