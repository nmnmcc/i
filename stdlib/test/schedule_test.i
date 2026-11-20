(
	testlib = (test) = i/test
	i/clock
	i/control
	i/fail
	i/list
	i/request
	i/schedule
	i/task
	i/time
)

ms = (n : Int) -> Int { n * 1000000 }

no_spread = (pattern : schedule.Pattern) -> schedule.Delay {
	schedule.Delay(pattern = pattern, spread = time.nanoseconds(0), seed = 0)
}

fixed_ms = (n : Int) -> schedule.Delay {
	no_spread(schedule.Fixed(delay = time.milliseconds(n)))
}

exponential = () -> schedule.Pattern {
	schedule.Exponential(
		first = time.milliseconds(1),
		factor = 2,
		max = time.milliseconds(5),
	)
}

// Adds one to a counter and gives the new count.
bump = (r : task.Ref(Int)) <- (own : task.Scheduler) -> Int {
	task.update(r, (n : Int) { (n + 1, n + 1) })
}

// Appends `n` to the list in `r`.
note = (r : task.Ref(List(Int)), n : Int) <- (own : task.Scheduler) -> () {
	task.update(r, (xs : List(Int)) { (list.append(xs, [n]), ()) })
}

// A clock whose sleeps are recorded into `slept`. A sleep is ready at once,
// except that the sleep numbered `block_at` (1-based) never ends; give 0 to
// never block.
fake_clock = (slept : task.Ref(List(Int)), block_at : Int) <- (
	own : task.Scheduler,
) -> clock.Clock {
	sleep = control.keep((n : Int) {
		note(slept, n)
		{ list.length(task.get(slept)) == block_at } |>
			(True) { task.sync(task.never()) }
			& (False) { () }
	})
	clock.Clock(
		monotonic_ns = () { request.ready(Ok(0)) },
		unix_ns = () { request.ready(Ok(0)) },
		sleep_ns = (n : Int) {
			done : Result((), Never) = fail.catch(() { sleep(n) })
			done |>
				(Ok(_)) { request.ready(Ok(())) }
				& (Error(_)) { panic("x") }
		},
	)
}

recording = (slept : task.Ref(List(Int))) <- (
	own : task.Scheduler,
) -> clock.Clock { fake_clock(slept, 0) }

// Yields until `ready()` holds, at most `n` times.
wait_until = (ready : () <- (task.Scheduler) -> Bool, n : Int) <- (
	own : task.Scheduler,
) -> () {
	{ ready() || n <= 0 } |>
		(True) { () }
		& (False) {
			task.yield()
			wait_until(ready, n - 1)
		}
}

// The sleeps of `retry(times, delay, always failing step)`.
retry_sleeps = (times : Int, delay : schedule.Delay) <- (
	own : task.Scheduler,
	runner : task.Runner,
	waiter : request.Await,
) -> List(Int) {
	slept = task.ref([])
	_ = fail.catch(() {
		schedule.retry(times, delay, () { fail.raise(0) }) <- (recording(slept))
	})
	task.get(slept)
}

test("schedule_retry_fixed_always_fails_test", () {
	task.run(() {
		slept = task.ref([])
		runs = task.ref(0)
		result = fail.catch(() {
			schedule.retry(3, fixed_ms(50), () {
				n = bump(runs)
				fail.raise(n)
			}) <- (recording(slept))
		})
		assert(task.get(runs) == 4)
		assert(task.get(slept) == [ms(50), ms(50), ms(50)])
		assert(result == Error(4))
	})
	()
})

test("schedule_retry_succeeds_on_second_attempt_test", () {
	task.run(() {
		slept = task.ref([])
		runs = task.ref(0)
		result = fail.catch(() {
			schedule.retry(3, fixed_ms(50), () {
				n = bump(runs)
				{ n < 2 } |> (True) { fail.raise(n) } & (False) { n * 10 }
			}) <- (recording(slept))
		})
		assert(result == Ok(20))
		assert(task.get(runs) == 2)
		assert(task.get(slept) == [ms(50)])
	})
	()
})

test("schedule_retry_exponential_is_capped_test", () {
	task.run(() {
		assert(
			retry_sleeps(
				4,
				no_spread(exponential()),
			) ==
				[ms(1), ms(2), ms(4), ms(5)],
		)
		// A factor below 1 counts as 1; the cap still holds.
		flat = schedule.Exponential(
			first = time.milliseconds(3),
			factor = -4,
			max = time.milliseconds(2),
		)
		assert(
			retry_sleeps(2, no_spread(flat)) == [ms(2), ms(2)],
		)
	})
	()
})

test("schedule_retry_jitter_is_bounded_and_reproducible_test", () {
	task.run(() {
		base = [ms(1), ms(2), ms(4), ms(5)]
		jittered = (seed : Int) -> schedule.Delay {
			schedule.Delay(
				pattern = exponential(),
				spread = time.milliseconds(1),
				seed = seed,
			)
		}
		first = retry_sleeps(4, jittered(7))
		second = retry_sleeps(4, jittered(7))
		assert(first == second)
		assert(list.length(first) == 4)
		assert(
			list.all(list.zip(first, base), (pair : (Int, Int)) {
				(got, want) = pair
				got >= want - ms(1) && got <= want + ms(1)
			}),
		)
		assert(
			retry_sleeps(
				4,
				schedule.Delay(
					pattern = exponential(),
					spread = time.nanoseconds(0),
					seed = 7,
				),
			) ==
				base,
		)
	})
	()
})

test("schedule_repeat_sleeps_between_runs_test", () {
	task.run(() {
		slept = task.ref([])
		runs = task.ref(0)
		schedule.repeat(3, fixed_ms(10), () {
			_ = bump(runs)
			()
		}) <- (recording(slept))
		assert(task.get(runs) == 3)
		assert(task.get(slept) == [ms(10), ms(10)])
	})
	()
})

test("schedule_zero_and_negative_times_test", () {
	task.run(() {
		slept = task.ref([])
		runs = task.ref(0)
		failing = () {
			n = bump(runs)
			fail.raise(n)
		}
		{
			assert(
				fail.catch(() { schedule.retry(0, fixed_ms(5), failing) }) ==
					Error(1),
			)
			assert(
				fail.catch(() { schedule.retry(-3, fixed_ms(5), failing) }) ==
					Error(2),
			)
			assert(task.get(runs) == 2)
			schedule.repeat(0, fixed_ms(5), () {
				_ = bump(runs)
				()
			})
			schedule.repeat(-1, fixed_ms(5), () {
				_ = bump(runs)
				()
			})
			assert(task.get(runs) == 2)
		} <- (recording(slept))
		assert(task.get(slept) == [])
	})
	()
})

test("schedule_cancel_during_sleep_stops_retry_test", () {
	task.run(() {
		slept = task.ref([])
		runs = task.ref(0)
		result = fail.catch(() {
			task.race(
				() {
					schedule.retry(5, fixed_ms(10), () {
						n = bump(runs)
						fail.raise(n)
					}) <- (fake_clock(slept, 2))
				},
				() {
					// Fail once the retry is blocked in its second sleep.
					wait_until(() { list.length(task.get(slept)) == 2 }, 1000)
					fail.raise(-1)
				},
			)
		})
		assert(result == Error(-1))
		// Give a retry that was wrongly left running some turns.
		wait_until(() { False }, 20)
		assert(task.get(runs) == 2)
		assert(task.get(slept) == [ms(10), ms(10)])
	})
	()
})

test("schedule_step_cancelled_is_not_retried_test", () {
	task.run(() {
		slept = task.ref([])
		runs = task.ref(0)
		result = fail.catch(() {
			task.race(
				() {
					schedule.retry(5, fixed_ms(10), () {
						_ = bump(runs)
						task.sync(task.never())
					}) <- (recording(slept))
				},
				() {
					wait_until(() { task.get(runs) == 1 }, 1000)
					fail.raise(-1)
				},
			)
		})
		assert(result == Error(-1))
		wait_until(() { False }, 20)
		assert(task.get(runs) == 1)
		assert(task.get(slept) == [])
	})
	()
})
