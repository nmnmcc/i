//// Retry and repeat schedules: run a step again after a failure, or a fixed
//// number of times, sleeping a planned delay between runs.
////
//// ```i
//// value = schedule.retry(
//// 	3,
//// 	schedule.Delay(
//// 		pattern = schedule.Fixed(delay = time.milliseconds(50)),
//// 		spread = time.milliseconds(0),
//// 		seed = 0,
//// 	),
//// 	() { fetch_once() },
//// )
//// ```
////
//// A `Delay` is a base `Pattern` plus jitter. `Fixed(delay)` waits `delay`
//// every time. `Exponential(first, factor, max)` waits `first`,
//// `first * factor`, `first * factor^2`, ... and never more than `max`.
//// A `factor` below 1 is treated as 1, so the delay stays at `first` (still
//// capped at `max`).
////
//// Exact delays. Every delay is computed in whole nanoseconds of `i/time`
//// with integer arithmetic only: nothing is rounded to milliseconds and no
//// floats are used. The sleep goes through `clock.sleep_ns` with exactly
//// that count. `delay_at` gives the delay for one attempt, so a program can
//// log what a schedule will wait, or run its own loop with the same policy
//// (for example retrying until a deadline instead of a fixed count).
////
//// Jitter. With `spread > 0`, the delay before attempt `a` (0-based: `a = 0`
//// is the sleep after the first failure) is `base + offset`, where
//// `offset = u - spread_ns` and `u` is the first draw of
//// `random.int(random.from_seed(seed + a), 2 * spread_ns + 1)`. So the offset
//// lies in `[-spread, +spread]`, is the same for the same `seed` and attempt
//// on every run and platform, and is 0 when `spread` is 0 (a negative
//// `spread` counts as 0). A jittered delay below zero is raised to zero.
//// The cap of `Exponential` applies to the base, before the offset.
////
//// Cancellation. A schedule only catches its step's `Fail(E)`. When the task
//// running a schedule is cancelled, during a step or during a sleep, the
//// schedule stops at once: it does not retry, does not run the step again and
//// does not sleep again. A step that ends by cancellation is never retried.
////
//// Needs. Both functions need the step's services, `Fail(E)`, and the
//// services of `clock.sleep_ns`: `clock.Clock`, `Fail(clock.ClockError)` and
//// `request.Await` (inside a task run, `task.Scheduler` and `task.Runner`
//// come with it). Use a fake `clock.Clock` in tests to record the sleeps
//// instead of waiting.
////
//// Stability: experimental. Since: next. Tracking: docs/design/std-v3.md.
(
	i/clock
	i/fail
	i/task
	i/time
	rng = i/random
	i/core
)

(
	Fixed(..)
	Exponential(..)
	Pattern(..)
	Delay(..)
	delay_at
	retry
	repeat
)

/// The same delay before every attempt.
Fixed = (delay = time.Duration)

/// `first`, then multiplied by `factor` for each later attempt, never more
/// than `max`. A `factor` below 1 counts as 1.
Exponential = (first = time.Duration, factor = Int, max = time.Duration)

/// The base delay of a schedule.
Pattern = Fixed | Exponential

/// A base pattern with jitter of up to `spread` either way, derived from
/// `seed` (see the module notes for the exact derivation).
Delay = (pattern = Pattern, spread = time.Duration, seed = Int)

_smaller = (a : Int, b : Int) -> Int {
	{ a < b } |> (True()) { a } & (False()) { b }
}

_larger = (a : Int, b : Int) -> Int {
	{ a > b } |> (True()) { a } & (False()) { b }
}

// `current * factor^steps`, stopping as soon as it reaches `cap`.
_grow = (current : Int, factor : Int, steps : Int, cap : Int) -> Int {
	{ steps <= 0 || current >= cap } |>
		(True()) { _smaller(current, cap) }
		& (False()) { _grow(current * factor, factor, steps - 1, cap) }
}

// The base delay before attempt `attempt`, in nanoseconds.
_base_ns = (pattern : Pattern, attempt : Int) -> Int {
	pattern |>
		(Fixed(..) = p) { time.to_nanoseconds(p.delay) }
		& (Exponential(..) = p) {
			_grow(
				time.to_nanoseconds(p.first),
				_larger(p.factor, 1),
				attempt,
				time.to_nanoseconds(p.max),
			)
		}
}

// The jitter offset for attempt `attempt`, in [-spread_ns, spread_ns].
_offset_ns = (spread_ns : Int, seed : Int, attempt : Int) -> Int {
	{ spread_ns <= 0 } |>
		(True()) { 0 }
		& (False()) {
			drawn = fail.catch(() {
				(u, _) = rng.int(
					rng.from_seed(seed + attempt),
					2 * spread_ns + 1,
				)
				u
			})
			drawn |>
				(Ok(u)) { u - spread_ns }
				& (Error(_)) { panic("schedule: invalid jitter bound") }
		}
}

/// The exact sleep before attempt `attempt`: the sleep after the first
/// failure is attempt 0. Gives the base delay of the pattern for that
/// attempt, capped, plus the jitter of that attempt, and never below zero.
/// Use it to see a schedule's delay vector without running it.
///
/// ```i
/// schedule.delay_at(schedule.Delay(
///   pattern = schedule.Fixed(delay = time.milliseconds(50)),
///   spread = time.nanoseconds(0), seed = 0,
/// ), 2)  // time.milliseconds(50)
/// ```
delay_at = (delay : Delay, attempt : Int) -> time.Duration {
	base = _base_ns(delay.pattern, attempt)
	offset = _offset_ns(time.to_nanoseconds(delay.spread), delay.seed, attempt)
	time.nanoseconds(_larger(base + offset, 0))
}

_sleep_before = (delay : Delay, attempt : Int) {
	clock.sleep_ns(
		time.to_nanoseconds(delay_at(delay, attempt)),
	)
}

_retry_from = <A, E, N>(
	left : Int,
	attempt : Int,
	delay : Delay,
	step : () <- (Fail(E), ..N) -> A
) <- (error : Fail(E), ..env : N) -> A {
	fail.catch(step) |>
		(Ok(value)) { value }
		& (Error(error)) {
			{ left <= 0 } |>
				(True()) { fail.raise(error) }
				& (False()) {
					_sleep_before(delay, attempt)
					_retry_from(left - 1, attempt + 1, delay, step)
				}
		}
}

/// Runs `step`; when it raises and retries remain, sleeps the delay for that
/// attempt and runs it again. `times` counts the retries after the first run:
/// `times = 0` runs `step` once, and a negative `times` counts as 0. Returns
/// the first value `step` gives.
/// Needs: the step's needs, `clock.Clock`, `Fail(clock.ClockError)`,
/// `request.Await` (from `task.Scheduler` and `task.Runner` in a task run).
/// Errors: when the retries are used up, the last error of `step`, unchanged;
/// a clock failure while sleeping. Cancellation is never retried.
///
/// ```i
/// example = () {
/// 	tries = task.ref(0)
/// 	value = schedule.retry(
/// 		3,
/// 		schedule.Delay(
/// 			pattern = schedule.Fixed(delay = time.milliseconds(50)),
/// 			spread = time.nanoseconds(0),
/// 			seed = 0,
/// 		),
/// 		() {
/// 			n = task.update(tries, (n : Int) { (n + 1, n + 1) })
/// 			{ n < 3 } |> (True()) { fail.raise("busy") } & (False()) { n }
/// 		},
/// 	)
/// 	assert(value == 3)
/// }
/// ```
retry = (times : Int, delay : Delay, step) {
	_retry_from(times, 0, delay, step)
}

_repeat_from = (left : Int, attempt : Int, delay : Delay, step) {
	_ = step()
	{ left <= 1 } |>
		(True()) { () }
		& (False()) {
			_sleep_before(delay, attempt)
			_repeat_from(left - 1, attempt + 1, delay, step)
		}
}

/// Runs `step` exactly `times` times, sleeping the delay for attempt `i`
/// between runs `i` and `i + 1`, and not after the last run. `times <= 0`
/// does nothing.
/// Needs: the step's needs, `clock.Clock`, `Fail(clock.ClockError)`,
/// `request.Await` (from `task.Scheduler` and `task.Runner` in a task run).
/// Errors: the first error of `step`, which stops the repetition; a clock
/// failure while sleeping.
///
/// ```i
/// example = () {
/// 	runs = task.ref(0)
/// 	schedule.repeat(
/// 		3,
/// 		schedule.Delay(
/// 			pattern = schedule.Fixed(delay = time.milliseconds(10)),
/// 			spread = time.nanoseconds(0),
/// 			seed = 0,
/// 		),
/// 		() { task.update(runs, (n : Int) { (n + 1, ()) }) },
/// 	)
/// 	assert(task.get(runs) == 3)
/// }
/// ```
repeat = (times : Int, delay : Delay, step) {
	{ times <= 0 } |>
		(True()) { () }
		& (False()) { _repeat_from(times, 0, delay, step) }
}
