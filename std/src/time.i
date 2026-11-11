//// Exact signed nanoseconds. Duration is a length; Instant is a Unix epoch
//// position. Both hide their constructors and have one Int representation.
//// TimeError exports DivisionByZero. Arithmetic is pure.
////
//// Interface:
//// nanoseconds/milliseconds/seconds/minutes/hours: (count: Int) -> Duration
//// to_nanoseconds: (duration: Duration) -> Int
//// add/subtract: (first: Duration, second: Duration) -> Duration
//// scale: (duration: Duration, by: Int) -> Duration
//// divide: (duration: Duration, by: Int) -> Result(Duration, TimeError)
//// compare: (first: Duration, second: Duration) -> Order
//// from_unix_nanoseconds: (count: Int) -> Instant
//// unix_nanoseconds: (instant: Instant) -> Int
//// shift: (instant: Instant, by: Duration) -> Instant
//// between: (start: Instant, end: Instant) -> Duration
//// compare_instants: (first: Instant, second: Instant) -> Order
////
//// T1: Unit constructors multiply by 1, 10^6, 10^9, 60*10^9 and 3600*10^9.
//// Every Int is accepted, including negative counts; no overflow or clamping.
//// T2: add, subtract and scale use exact integer arithmetic. divide rejects
//// zero with Error(DivisionByZero), otherwise rounds toward zero, including
//// negative divisors. compare uses numeric order.
//// T3: Instant zero is 1970-01-01T00:00:00Z. between(a,b) is b-a, shift adds
//// a duration, and unix_nanoseconds reverses from_unix_nanoseconds. Instants
//// cannot be added to one another. Equality compares canonical payloads.
////
//// Laws: to_nanoseconds(nanoseconds(n)) = n; shift(a,between(a,b)) = b;
//// between(a,shift(a,d)) = d; add(d,subtract(e,d)) = e. Truncating divide
//// can lose a remainder: scaling its result back need not give the input.
//// Calendar days, months and years are not duration units. hours(24) is a
//// fixed 86,400 seconds. Operations use O(1) integer operations; bit costs
//// depend on integer size. Observation functions use i/clock.

(
	i/clock
	std/int
	(Order) = std/order
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
)

Duration {
	Duration(Int)
}

Instant {
	Instant(Int)
}

TimeError {
	DivisionByZero
}

/// Makes a duration from an exact count of nanoseconds.
nanoseconds = (count: Int) -> Duration { Duration(count) }

/// Makes a duration from an exact count of milliseconds.
milliseconds = (count: Int) -> Duration { Duration(count * 1000000) }

/// Makes a duration from an exact count of seconds.
seconds = (count: Int) -> Duration { Duration(count * 1000000000) }

/// Makes a duration from an exact count of minutes.
minutes = (count: Int) -> Duration { Duration(count * 60000000000) }

/// Makes a duration from an exact count of hours.
hours = (count: Int) -> Duration { Duration(count * 3600000000000) }

/// Returns the signed nanosecond payload.
to_nanoseconds = (duration: Duration) -> Int {
	Duration(count) = duration
	count
}

add = (first: Duration, second: Duration) -> Duration {
	Duration(to_nanoseconds(first) + to_nanoseconds(second))
}

subtract = (first: Duration, second: Duration) -> Duration {
	Duration(to_nanoseconds(first) - to_nanoseconds(second))
}

scale = (duration: Duration, by: Int) -> Duration {
	Duration(to_nanoseconds(duration) * by)
}

/// Divides toward zero, or returns DivisionByZero for a zero divisor.
divide = (duration: Duration, by: Int) -> Result(Duration, TimeError) {
	{ by == 0 } |> {
		True  -> Error(DivisionByZero)
		False -> Ok(Duration(to_nanoseconds(duration) / by))
	}
}

compare = (first: Duration, second: Duration) -> Order {
	int.compare(to_nanoseconds(first), to_nanoseconds(second))
}

/// Makes an epoch position from signed Unix nanoseconds.
from_unix_nanoseconds = (count: Int) -> Instant { Instant(count) }

unix_nanoseconds = (instant: Instant) -> Int {
	Instant(count) = instant
	count
}

shift = (instant: Instant, by: Duration) -> Instant {
	Instant(unix_nanoseconds(instant) + to_nanoseconds(by))
}

/// Returns end minus start, so reversing the arguments reverses the sign.
between = (start: Instant, end: Instant) -> Duration {
	Duration(unix_nanoseconds(end) - unix_nanoseconds(start))
}

compare_instants = (first: Instant, second: Instant) -> Order {
	int.compare(unix_nanoseconds(first), unix_nanoseconds(second))
}

/// Reads the Unix clock once.
now = () { Instant(clock.unix_ns()) }

/// Waits for the given signed duration.
sleep = (duration: Duration) { clock.sleep_ns(to_nanoseconds(duration)) }

/// Measures one call using the monotonic clock.
measure = (body) {
	start = clock.monotonic_ns()
	value = body()
	end = clock.monotonic_ns()
	(value, Duration(end - start))
}
