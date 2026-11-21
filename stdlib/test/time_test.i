(
	i/fail
	testlib = (test) = i/test
	i/time
	i/list
	i/task
	i/resource
	i/clock
	i/request
	i/core
	(Less, Equal, Greater) = i/order
)

// Unit conversion requires every signed Int and exact unit multipliers.
test("time_units", () {
	list.each([
		-100000000000000000000,
		-2,
		-1,
		0,
		1,
		2,
		100000000000000000000,
	], (n) {
		assert(
			time.to_nanoseconds(time.nanoseconds(n)) == n,
		)
		assert(
			time.to_nanoseconds(time.milliseconds(n)) == n * 1000000,
		)
		assert(
			time.to_nanoseconds(time.seconds(n)) == n * 1000000000,
		)
		assert(
			time.to_nanoseconds(time.minutes(n)) == n * 60000000000,
		)
		assert(
			time.to_nanoseconds(time.hours(n)) == n * 3600000000000,
		)
		assert(time.minutes(n) == time.seconds(n * 60))
		assert(time.hours(n) == time.minutes(n * 60))
	})
	assert(
		time.to_nanoseconds(time.seconds(-2)) == -2000000000,
	)
	()
})

// Duration arithmetic uses truncating division, for either sign of the divisor.
test("time_arithmetic", () {
	assert(
		{
			e4_call_arg_0 = time.nanoseconds(-5)
			e4_call_arg_1 = 2
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(time.nanoseconds(-2)),
	)
	assert(
		{
			e4_call_arg_0 = time.nanoseconds(5)
			e4_call_arg_1 = -2
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(time.nanoseconds(-2)),
	)
	assert(
		{
			e4_call_arg_0 = time.nanoseconds(-5)
			e4_call_arg_1 = -2
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(time.nanoseconds(2)),
	)
	assert(
		{
			e4_call_arg_0 = time.nanoseconds(5)
			e4_call_arg_1 = 2
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(time.nanoseconds(2)),
	)
	assert(
		{
			e4_call_arg_0 = time.seconds(1)
			e4_call_arg_1 = 0
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Error(time.DivisionByZero()),
	)
	assert(
		{
			e4_call_arg_0 = time.nanoseconds(0)
			e4_call_arg_1 = 0
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Error(time.DivisionByZero()),
	)
	assert(
		{
			e4_call_arg_0 = time.nanoseconds(0)
			e4_call_arg_1 = -2
			fail.catch(() { time.divide(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(time.nanoseconds(0)),
	)
	assert(
		time.scale(time.nanoseconds(-2), 2) != time.nanoseconds(-5),
	)
	list.each([-7, -1, 0, 1, 7], (a) {
		list.each([-9, -1, 0, 1, 9], (b) {
			d = time.nanoseconds(a)
			e = time.nanoseconds(b)
			assert(
				time.to_nanoseconds(time.add(d, e)) == a + b,
			)
			assert(
				time.to_nanoseconds(time.subtract(d, e)) == a - b,
			)
			assert(
				time.to_nanoseconds(time.scale(d, b)) == a * b,
			)
			// The duration arithmetic law.
			assert(
				time.add(d, time.subtract(e, d)) == e,
			)
		})
	})
	assert(
		time.compare(time.nanoseconds(-1), time.nanoseconds(0)) == Less(),
	)
	assert(
		time.compare(time.seconds(1), time.milliseconds(1000)) == Equal(),
	)
	assert(
		time.compare(time.seconds(1), time.nanoseconds(1)) == Greater(),
	)
	()
})

// The epoch and inverse laws apply to negative and arbitrarily large counts.
test("time_instants", () {
	epoch = time.from_unix_nanoseconds(0)
	assert(
		time.shift(time.from_unix_nanoseconds(-1), time.nanoseconds(1)) ==
			epoch,
	)
	assert(
		time.between(
			time.from_unix_nanoseconds(7),
			time.from_unix_nanoseconds(2),
		) ==
			time.nanoseconds(-5),
	)
	list.each([-100000000000000000000, -1, 0, 1, 100000000000000000000], (a) {
		start = time.from_unix_nanoseconds(a)
		assert(time.unix_nanoseconds(start) == a)
		assert(
			time.from_unix_nanoseconds(time.unix_nanoseconds(start)) == start,
		)
		list.each([-100, -1, 0, 1, 100], (b) {
			end = time.from_unix_nanoseconds(b)
			duration = time.nanoseconds(b)
			assert(
				time.shift(start, time.between(start, end)) == end,
			)
			assert(
				time.between(start, time.shift(start, duration)) == duration,
			)
		})
	})
	assert(
		time.compare_instants(time.from_unix_nanoseconds(-1), epoch) == Less(),
	)
	assert(
		time.compare_instants(epoch, time.from_unix_nanoseconds(0)) == Equal(),
	)
	assert(
		time.compare_instants(time.from_unix_nanoseconds(1), epoch) ==
			Greater(),
	)
	()
})

// Durations from huge counts such as hours(10^100) must not overflow, since Int
// is unbounded.
test("time_huge_counts", () {
	huge = 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
	duration = time.hours(huge)
	start = time.from_unix_nanoseconds(-1)
	assert(time.to_nanoseconds(duration) == huge * 3600000000000)
	assert(
		time.shift(time.shift(start, duration), time.scale(duration, -1)) ==
			start,
	)
	assert(
		time.between(start, time.shift(start, duration)) == duration,
	)
	assert(
		time.to_nanoseconds(
			time.add(
				time.nanoseconds(9223372036854775807),
				time.nanoseconds(1),
			),
		) ==
			9223372036854775808,
	)
	assert(
		time.to_nanoseconds(
			time.scale(time.nanoseconds(-9223372036854775808), -1),
		) ==
			9223372036854775808,
	)
	()
})
// Duration and Instant stay nominally distinct, with private constructors.

// A clock that fails if anything sleeps on it: the tests that must not sleep
// use it to prove that no sleep happened.
unused_clock : clock.Clock = clock.Clock(
	monotonic_ns = () { request.ready(Ok(0)) },
	unix_ns = () { request.ready(Ok(0)) },
	sleep_ns = (n) { request.ready(Error("unused")) },
)

late = () { task.sync(task.never()) }

test("timeout_body_wins_test", () {
	// The timer is a real hour away and is cancelled when the body returns.
	value = time.timeout(time.hours(1), () {
		task.yield()
		7
	})
	assert(value == 7)
	()
})

// The timer fires while the body is stuck. The body's `ensure` final sets a
// flag, and the flag must be set by the time `timeout` returns: that is R5's
// "return only after their finals end", observed.
test("timeout_timer_wins_test", () {
	released = task.ref(False())
	(r, elapsed) = time.measure(() {
		fail.catch(() {
			time.timeout(time.milliseconds(20), () {
				resource.ensure(
					() { late() },
					() { task.set(released, True()) },
				)
			})
		})
	})
	assert(r == Error(time.Timeout(after = time.milliseconds(20))))
	assert(task.get(released))
	// The timer cannot fire before its deadline. No upper bound: load.
	assert(time.to_nanoseconds(elapsed) >= 20000000)
	()
})

test("timeout_expired_test", () {
	ran = task.ref(False())
	{
		zero = fail.catch(() {
			time.timeout(time.nanoseconds(0), () {
				task.set(ran, True())
				1
			})
		})
		assert(zero == Error(time.Timeout(after = time.nanoseconds(0))))
		negative = fail.catch(() {
			time.timeout(time.nanoseconds(-5), () {
				task.set(ran, True())
				1
			})
		})
		assert(negative == Error(time.Timeout(after = time.nanoseconds(-5))))
		// A zero or negative duration never starts the body and never sleeps.
		assert(task.get(ran) == False())
		()
	} <- (unused_clock)
})

test("timeout_body_error_test", () {
	// The deadline is only an upper bound: the body returns in two scheduler
	// turns (one yield, then the raise), while the timer is an hour away and
	// is cancelled when the body returns. The body error is the only possible
	// outcome, so this does not depend on which child wins a race.
	r = fail.catch(() {
		time.timeout(time.hours(1), () {
			task.yield()
			fail.raise("boom")
		})
	})
	// A body error comes back as-is, never as this call's Timeout.
	assert(r == Error("boom"))
	assert(r != Error(time.Timeout(after = time.hours(1))))
	()
})

// A clock that cannot install the timer: `sleep_ns` is already failed when it
// is handed back, so the timer loses no matter how the scheduler orders it.
failing_clock : clock.Clock = clock.Clock(
	monotonic_ns = () { request.ready(Ok(0)) },
	unix_ns = () { request.ready(Ok(0)) },
	sleep_ns = (n) { request.ready(Error("boom")) },
)

// The clock-failure path: the body is blocked forever, so the only way
// `timeout` can return is the failed timer. It must cancel the body, run the
// body's final, and then return the clock's error, not a Timeout.
test("timeout_clock_failure_test", () {
	ran = task.ref(False())
	{
		r = fail.catch(() {
			time.timeout(time.hours(1), () {
				resource.ensure(
					() { task.sync(task.never()) },
					() { task.set(ran, True()) },
				)
			})
		})
		assert(
			r == Error(clock.ClockError(operation = "sleep_ns", code = "boom")),
		)
		// R5: the body's final ran before `timeout` returned.
		assert(task.get(ran))
		// The clock's error is not this call's Timeout.
		assert(r != Error(time.Timeout(after = time.hours(1))))
		()
	} <- (failing_clock)
})

test("timeout_bad_deadline_test", () {
	ran = task.ref(False())
	too_big = time.nanoseconds(9223372036854775808)
	{
		r = fail.catch(() {
			time.timeout(too_big, () {
				task.set(ran, True())
				1
			})
		})
		assert(r == Error(time.BadDeadline(after = too_big)))
		// The body never starts.
		assert(task.get(ran) == False())
		()
	} <- (unused_clock)
})
