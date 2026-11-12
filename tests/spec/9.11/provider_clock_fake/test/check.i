(
	testlib = (test) = std/test
	i/clock
)

test("fake_time", () { assert(clock.monotonic_ns() == 42) })
