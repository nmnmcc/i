//// Exact signed nanoseconds. Duration is a length; Instant is a Unix epoch
//// position. Both hide their constructors and have one Int representation.
//// TimeError exports DivisionByZero. Division failures use Fail(TimeError).
////
//// Interface:
//// nanoseconds/milliseconds/seconds/minutes/hours: (count: Int) -> Duration
//// to_nanoseconds: (duration: Duration) -> Int
//// add/subtract: (first: Duration, second: Duration) -> Duration
//// scale: (duration: Duration, by: Int) -> Duration
//// divide: (duration: Duration, by: Int) <- (Fail(TimeError)) -> Duration
//// compare: (first: Duration, second: Duration) -> Order
//// from_unix_nanoseconds: (count: Int) -> Instant
//// unix_nanoseconds: (instant: Instant) -> Int
//// shift: (instant: Instant, by: Duration) -> Instant
//// between: (start: Instant, end: Instant) -> Duration
//// compare_instants: (first: Instant, second: Instant) -> Order
////
//// Unit constructors multiply by 1, 10^6, 10^9, 60*10^9 and 3600*10^9.
//// Every Int is accepted, including negative counts; no overflow or clamping.
//// add, subtract and scale use exact integer arithmetic. divide rejects
//// zero by raising DivisionByZero, otherwise rounds toward zero, including
//// negative divisors. compare uses numeric order.
//// Instant zero is 1970-01-01T00:00:00Z. between(a,b) is b-a, shift adds
//// a duration, and unix_nanoseconds reverses from_unix_nanoseconds. Instants
//// cannot be added to one another. Equality compares canonical payloads.
////
//// Laws: to_nanoseconds(nanoseconds(n)) = n; shift(a,between(a,b)) = b;
//// between(a,shift(a,d)) = d; add(d,subtract(e,d)) = e. Truncating divide
//// can lose a remainder: scaling its result back need not give the input.
//// timeout(after, body) runs body and gives its value, or raises
//// Timeout(after) when the deadline passes first (a zero or negative after
//// raises at once without running body).
////
//// Calendar days, months and years are not duration units. hours(24) is a
//// fixed 86,400 seconds. Operations use O(1) integer operations; bit costs
//// depend on integer size. Observation functions use i/clock.
(
	i/fail
	i/clock
	i/int
	i/task
	(Order) = i/order
)

(
	Duration
	Instant
	TimeError(..)
	nanoseconds
	milliseconds
	seconds
	minutes
	hours
	to_nanoseconds
	add
	subtract
	scale
	divide
	compare
	from_unix_nanoseconds
	unix_nanoseconds
	shift
	between
	compare_instants
	now
	sleep
	measure
	Timeout(..)
	BadDeadline(..)
	timeout
)

Duration = (Int)

Instant = (Int)

DivisionByZero = ()
TimeError = DivisionByZero

/// Makes a duration from an exact count of nanoseconds.
nanoseconds = (count : Int) -> Duration { Duration(count) }

/// Makes a duration from an exact count of milliseconds.
milliseconds = (count : Int) -> Duration { Duration(count * 1000000) }

/// Makes a duration from an exact count of seconds.
seconds = (count : Int) -> Duration { Duration(count * 1000000000) }

/// Makes a duration from an exact count of minutes.
minutes = (count : Int) -> Duration { Duration(count * 60000000000) }

/// Makes a duration from an exact count of hours.
hours = (count : Int) -> Duration { Duration(count * 3600000000000) }

/// Returns the signed nanosecond payload.
to_nanoseconds = (duration : Duration) -> Int {
	Duration(count) = duration
	count
}

add = (first : Duration, second : Duration) -> Duration {
	Duration(to_nanoseconds(first) + to_nanoseconds(second))
}

subtract = (first : Duration, second : Duration) -> Duration {
	Duration(to_nanoseconds(first) - to_nanoseconds(second))
}

scale = (duration : Duration, by : Int) -> Duration {
	Duration(to_nanoseconds(duration) * by)
}

_divide_result = (duration : Duration, by : Int) -> Result(
	Duration,
	TimeError,
) {
	{ by == 0 } |>
		(True()) { Error(DivisionByZero()) }
		& (False()) { Ok(Duration(to_nanoseconds(duration) / by)) }
}

compare = (first : Duration, second : Duration) -> Order {
	int.compare(to_nanoseconds(first), to_nanoseconds(second))
}

/// Makes an epoch position from signed Unix nanoseconds.
from_unix_nanoseconds = (count : Int) -> Instant { Instant(count) }

unix_nanoseconds = (instant : Instant) -> Int {
	Instant(count) = instant
	count
}

shift = (instant : Instant, by : Duration) -> Instant {
	Instant(unix_nanoseconds(instant) + to_nanoseconds(by))
}

/// Returns end minus start, so reversing the arguments reverses the sign.
between = (start : Instant, end : Instant) -> Duration {
	Duration(unix_nanoseconds(end) - unix_nanoseconds(start))
}

compare_instants = (first : Instant, second : Instant) -> Order {
	int.compare(unix_nanoseconds(first), unix_nanoseconds(second))
}

/// Reads the Unix clock once.
now = () { Instant(clock.unix_ns()) }

/// Waits for the given signed duration.
sleep = (duration : Duration) { clock.sleep_ns(to_nanoseconds(duration)) }

/// Measures one call using the monotonic clock.
measure = (body) {
	start = clock.monotonic_ns()
	value = body()
	end = clock.monotonic_ns()
	(value, Duration(end - start))
}

/// Divides toward zero, or returns DivisionByZero for a zero divisor.
///
/// Reports failure through `Fail(TimeError)`.
divide = (duration : Duration, by : Int) -> Duration {
	fail.from_result(_divide_result(duration, by))
}

/// The deadline passed before the body ended. `after` is the duration
/// `timeout` was given, so the timeout is distinguishable from the body's
/// own errors.
Timeout = (after = Duration)

/// `timeout` was given a deadline the clock cannot hold: more than
/// `max_timeout` of about 292 years. The body is not started.
BadDeadline = (after = Duration)

/// The largest deadline `timeout` accepts, in nanoseconds: 2^63 - 1.
max_timeout = 9223372036854775807

/// What the race below gives: the body's value, the deadline, or a clock
/// that could not install the timer.
Ran(A) = (A)
Expired = ()
ClockFailed = (clock.ClockError)
Outcome(A) = Ran(A) | Expired | ClockFailed

/// Runs `body` and gives its value, or raises `Timeout(after = after)` when
/// `after` passes first. A zero or negative `after` raises at once and does
/// not start the body.
///
/// The deadline must fit the clock: an `after` above `max_timeout` (2^63 - 1
/// nanoseconds, about 292 years) raises `BadDeadline(after = after)` before
/// the body starts, rather than failing later inside the timer.
///
/// The body runs in a task of its own and is cancelled when the deadline
/// passes; `timeout` returns only after that task has finished its cleanup,
/// so a body must be safe to cancel. A body error comes back unchanged.
///
/// If the clock cannot install the timer, `timeout` cancels the body and
/// raises the clock's error once the body has unwound; it never waits for a
/// blocked body to finish on its own.
///
/// That path is covered by `timeout_clock_failure_test` in `time_test`: a
/// fake clock whose `sleep_ns` is already failed while the body blocks
/// forever, so the clock's error is the only outcome, and the test checks
/// that the body's `ensure` final ran before `timeout` returned. Awaiting an
/// already-failed fake request in a child task used to hang; #65 fixed that,
/// which is what makes the test possible.
///
/// Needs `Scheduler`, `Runner` and `clock.Clock`.
/// Errors: the body's own `E`, `Timeout(after = after)`,
/// `BadDeadline(after = after)`, and `clock.ClockError`.
///
/// ```i
/// task.run(() {
///   fail.catch(() { time.timeout(time.seconds(1), () { work() }) }) |>
///     (Ok(value)) { value }
///     & (Error(time.Timeout(_))) { fallback }
/// })
/// ```
timeout = (after : Duration, body) {
	Duration(nanoseconds) = after
	{ nanoseconds > max_timeout } |>
		(True()) { fail.raise(BadDeadline(after = after)) }
		& (False()) { () }
	{ nanoseconds <= 0 } |>
		(True()) { fail.raise(Timeout(after = after)) }
		& (False()) {
			// The timer reports a clock failure as a value, not as a child
			// failure: a failure would let the race wait for the body (R2),
			// which may never end. As a value it commits the race, cancels
			// the body and returns after the body's finals.
			outcome = task.race(
				() { Ran(body()) },
				() {
					// Let the body take its first step first: a timer that
					// completes before the body has started would have the
					// race cancel a child that never ran.
					task.yield()
					fail.catch(() { clock.sleep_ns(nanoseconds) }) |>
						(Ok(_)) { Expired() }
						& (Error(error)) { ClockFailed(error) }
				},
			)
			outcome |>
				(Ran(value)) { value }
				& (Expired()) { fail.raise(Timeout(after = after)) }
				& (ClockFailed(error)) { fail.raise(error) }
		}
}
