(
	testlib = (test) = std/test
	i/fail
)

Clock = (now = () -> Int)

Late = (by = Int)

// Inferred: (deadline: Int) -> Int ^ Late ! Clock
remaining = (deadline) <- (clock: Clock) {
	deadline - clock.now() |>
		(left ? left < 0) { fail.raise(Late(by = 0 - left)) }
		| (left) { left }
}

test("a test gives its own clock", () {
	{
		assert(remaining(42) == 2)
		late = fail.catch(() { remaining(30) })
		expected = Error(Late(by = 10))
		assert(late == expected)
	} <- (Clock(now = () { 40 }))
})
