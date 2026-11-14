(
	testlib = (test) = std/test
	std/time
	std/list
	(Less, Equal, Greater) = std/order
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
	Nil
})

// Duration arithmetic uses truncating division, for either sign of the divisor.
test("time_arithmetic", () {
	assert(
		time.divide(time.nanoseconds(-5), 2) == Ok(time.nanoseconds(-2)),
	)
	assert(
		time.divide(time.nanoseconds(5), -2) == Ok(time.nanoseconds(-2)),
	)
	assert(
		time.divide(time.nanoseconds(-5), -2) == Ok(time.nanoseconds(2)),
	)
	assert(
		time.divide(time.nanoseconds(5), 2) == Ok(time.nanoseconds(2)),
	)
	assert(
		time.divide(time.seconds(1), 0) == Error(time.DivisionByZero),
	)
	assert(
		time.divide(time.nanoseconds(0), 0) == Error(time.DivisionByZero),
	)
	assert(
		time.divide(time.nanoseconds(0), -2) == Ok(time.nanoseconds(0)),
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
		time.compare(time.nanoseconds(-1), time.nanoseconds(0)) == Less,
	)
	assert(
		time.compare(time.seconds(1), time.milliseconds(1000)) == Equal,
	)
	assert(
		time.compare(time.seconds(1), time.nanoseconds(1)) == Greater,
	)
	Nil
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
		time.compare_instants(time.from_unix_nanoseconds(-1), epoch) == Less,
	)
	assert(
		time.compare_instants(epoch, time.from_unix_nanoseconds(0)) == Equal,
	)
	assert(
		time.compare_instants(time.from_unix_nanoseconds(1), epoch) == Greater,
	)
	Nil
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
	Nil
})
// Duration and Instant stay nominally distinct, with private constructors.
