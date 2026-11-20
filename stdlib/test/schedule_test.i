(
	testlib = (test) = i/test
	i/fail
	i/schedule
	i/task
	i/time
)

no_spread = (pattern : schedule.Pattern) -> schedule.Delay {
	schedule.Delay(pattern = pattern, spread = time.nanoseconds(0), seed = 0)
}

fixed = (ms : Int) -> schedule.Delay {
	no_spread(schedule.Fixed(delay = time.milliseconds(ms)))
}

bump = (r : task.Ref(Int)) <- (own : task.Scheduler) -> Int {
	task.update(r, (n : Int) { (n + 1, n + 1) })
}

test("schedule_fixed_and_exponential_delays_test", () {
	f = fixed(50)
	assert(schedule.delay_at(f, 0) == time.milliseconds(50))
	assert(schedule.delay_at(f, 7) == time.milliseconds(50))
	e = no_spread(
		schedule.Exponential(
			first = time.milliseconds(1),
			factor = 2,
			max = time.milliseconds(5),
		),
	)
	assert(schedule.delay_at(e, 0) == time.milliseconds(1))
	assert(schedule.delay_at(e, 1) == time.milliseconds(2))
	assert(schedule.delay_at(e, 2) == time.milliseconds(4))
	assert(schedule.delay_at(e, 3) == time.milliseconds(5))
	assert(schedule.delay_at(e, 9) == time.milliseconds(5))
	// A factor below one stays at the first delay, still capped by `max`.
	slow = no_spread(
		schedule.Exponential(
			first = time.milliseconds(2),
			factor = -4,
			max = time.milliseconds(9),
		),
	)
	assert(schedule.delay_at(slow, 5) == time.milliseconds(2))
	()
})

test("schedule_jitter_is_bounded_and_reproducible_test", () {
	delay = (seed : Int) -> schedule.Delay {
		schedule.Delay(
			pattern = schedule.Fixed(delay = time.milliseconds(10)),
			spread = time.milliseconds(1),
			seed = seed,
		)
	}
	base = time.to_nanoseconds(time.milliseconds(10))
	spread = time.to_nanoseconds(time.milliseconds(1))
	at = (d : schedule.Delay, n : Int) -> Int {
		time.to_nanoseconds(schedule.delay_at(d, n))
	}
	first = delay(7)
	assert(at(first, 0) >= base - spread && at(first, 0) <= base + spread)
	assert(at(first, 1) >= base - spread && at(first, 1) <= base + spread)
	assert(at(first, 2) >= base - spread && at(first, 2) <= base + spread)
	assert(
		at(first, 0) == at(delay(7), 0),
	)
	assert(
		at(first, 1) == at(delay(7), 1),
	)
	assert(
		at(first, 2) == at(delay(7), 2),
	)
	assert(
		at(fixed(10), 4) == base,
	)
	()
})

test("schedule_retry_runs_once_per_attempt_test", () {
	runs = task.ref(0)
	(outcome, elapsed) = time.measure(() {
		fail.catch(() {
			schedule.retry(3, fixed(5), () {
				_ = bump(runs)
				fail.raise("no")
			})
		})
	})
	assert(outcome == Error("no"))
	assert(task.get(runs) == 4)
	// Three 5 ms sleeps cannot take less than 15 ms. The upper bound is left
	// out: this machine's load makes elapsed-time ceilings flaky.
	assert(time.to_nanoseconds(elapsed) >= 15000000)
	()
})

test("schedule_retry_succeeds_on_second_attempt_test", () {
	runs = task.ref(0)
	value = schedule.retry(3, fixed(1), () {
		n = bump(runs)
		{ n == 1 } |> (True) { fail.raise("first") } & (False) { n }
	})
	assert(value == 2)
	assert(task.get(runs) == 2)
	()
})

test("schedule_repeat_runs_times_test", () {
	runs = task.ref(0)
	schedule.repeat(3, fixed(1), () {
		_ = bump(runs)
		()
	})
	assert(task.get(runs) == 3)
	()
})

test("schedule_zero_and_negative_times_test", () {
	one = task.ref(0)
	r = fail.catch(() {
		schedule.retry(0, fixed(1), () {
			_ = bump(one)
			fail.raise("no")
		})
	})
	assert(r == Error("no"))
	assert(task.get(one) == 1)
	negative = task.ref(0)
	_ = fail.catch(() {
		schedule.retry(-2, fixed(1), () {
			_ = bump(negative)
			fail.raise("no")
		})
	})
	assert(task.get(negative) == 1)
	none = task.ref(0)
	schedule.repeat(0, fixed(1), () {
		_ = bump(none)
		()
	})
	schedule.repeat(-1, fixed(1), () {
		_ = bump(none)
		()
	})
	assert(task.get(none) == 0)
	()
})

test("schedule_error_stops_repeat_test", () {
	runs = task.ref(0)
	r = fail.catch(() {
		schedule.repeat(3, fixed(1), () {
			_ = bump(runs)
			fail.raise("stop")
		})
	})
	assert(r == Error("stop"))
	assert(task.get(runs) == 1)
	()
})

// A retry cancelled during its sleep does not run the step again.
test("schedule_cancel_during_sleep_stops_retry_test", () {
	task.run(() {
		runs = task.ref(0)
		task.scope((_) {
			task.race(
				() {
					_ = fail.catch(() {
						schedule.retry(3, fixed(10000), () {
							_ = bump(runs)
							fail.raise("no")
						})
					})
					()
				},
				() {
					task.yield()
					task.yield()
					task.yield()
				},
			)
			()
		})
		assert(task.get(runs) == 1)
		()
	})
	()
})

// A step cancelled while it runs is not retried.
test("schedule_cancelled_step_is_not_retried_test", () {
	task.run(() {
		runs = task.ref(0)
		task.scope((_) {
			task.race(
				() {
					schedule.retry(3, fixed(1), () {
						_ = bump(runs)
						task.sync(task.never())
					})
					()
				},
				() {
					task.yield()
					task.yield()
					task.yield()
				},
			)
			()
		})
		assert(task.get(runs) == 1)
		()
	})
	()
})
