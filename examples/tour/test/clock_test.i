(
	testlib = (test) = std/test
	i/fail
)

Clock {
	Clock(now: () -> Int)
}

Late {
	Late(by: Int)
}

// Inferred: (deadline: Int) -> Int ^ Late ! Clock
remaining = (deadline, clock ! Clock) {
	deadline - clock.now() |> {
		left ? left < 0 -> fail.raise(Late(by: 0 - left))
		left            -> left
	}
}

test("a test gives its own clock", () {
	! Clock = Clock(now: () { 40 })
	assert(remaining(42) == 2)
	late = fail.catch(() { remaining(30) })
	assert(late == Error(Late(by: 10)))
})
