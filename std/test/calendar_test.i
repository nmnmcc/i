(
	std/calendar
	std/time
	std/list
	std/int
)

valid_date = (year: Int, month: Int, day: Int) -> calendar.Date ! Assert {
	calendar.date(year, month, day) |> {
		Ok(value) -> value
		Error(_)  -> panic("invalid test date")
	}
}

valid_time = (
	hour: Int
	minute: Int
	second: Int
	nanosecond: Int
) -> calendar.TimeOfDay ! Assert {
	calendar.time_of_day(hour, minute, second, nanosecond) |> {
		Ok(value) -> value
		Error(_)  -> panic("invalid test time")
	}
}

valid_offset = (minutes: Int) -> calendar.Offset ! Assert {
	calendar.offset(minutes) |> {
		Ok(value) -> value
		Error(_)  -> panic("invalid test offset")
	}
}

// C1/C2: validate in component order, without normalization.
@test
calendar_validation = () {
	list.each(
		[(1900, False), (2000, True), (0, True), (-100, False), (-400, True)]
		(pair) {
			(year, expected) = pair
			assert(calendar.is_leap_year(year) == expected)
		}
	)
	assert(calendar.date(1900, 2, 29) == Error(calendar.BadDay))
	assert(calendar.date(2000, 2, 29) == Ok(valid_date(2000, 2, 29)))
	assert(calendar.date(2024, 13, 0) == Error(calendar.BadMonth))
	assert(calendar.date(2024, 0, 1) == Error(calendar.BadMonth))
	assert(calendar.date(2024, 1, 0) == Error(calendar.BadDay))
	assert(calendar.date(2024, 4, 31) == Error(calendar.BadDay))
	assert(calendar.time_of_day(24, 0, 0, 0) == Error(calendar.BadHour))
	assert(calendar.time_of_day(-1, 60, 60, -1) == Error(calendar.BadHour))
	assert(calendar.time_of_day(0, -1, 60, -1) == Error(calendar.BadMinute))
	assert(calendar.time_of_day(0, 60, 60, -1) == Error(calendar.BadMinute))
	assert(calendar.time_of_day(0, 0, 60, 0) == Error(calendar.BadSecond))
	assert(calendar.time_of_day(0, 0, -1, -1) == Error(calendar.BadSecond))
	assert(calendar.time_of_day(0, 0, 0, -1) == Error(calendar.BadNanosecond))
	assert(
		calendar.time_of_day(0, 0, 0, 1000000000) ==
			Error(calendar.BadNanosecond)
	)
	assert(
		calendar.time_parts(valid_time(23, 59, 59, 999999999)) ==
			(23, 59, 59, 999999999)
	)
	assert(calendar.date_parts(valid_date(-1, 1, 2)) == (-1, 1, 2))
	list.each([-1440, 1440], (n) {
		assert(calendar.offset(n) == Error(calendar.BadOffset))
	})
	list.each([-1439, 0, 1439], (n) {
		assert(calendar.offset_minutes(valid_offset(n)) == n)
	})
	assert(calendar.utc() == valid_offset(0))
	list.each(list.range(1, 12), (month) {
		expected = month |> {
			2              -> 29
			4 | 6 | 9 | 11 -> 30
			_              -> 31
		}
		assert(calendar.days_in_month(2000, month) == Ok(expected))
	})
	assert(calendar.days_in_month(2000, -1) == Error(calendar.BadMonth))
	assert(calendar.days_in_month(2000, 13) == Error(calendar.BadMonth))
	Nil
}

// Review's independent day table follows C1/C3 and C4's weekday origin.
@test
calendar_known_days = () {
	cases = [
		(-400, 2, 29, -865566, calendar.Tuesday)
		(-100, 3, 1, -755993, calendar.Thursday)
		(-1, 1, 1, -719893, calendar.Friday)
		(0, 1, 1, -719528, calendar.Saturday)
		(0, 2, 29, -719469, calendar.Tuesday)
		(1, 1, 1, -719162, calendar.Monday)
		(1900, 3, 1, -25508, calendar.Thursday)
		(2000, 3, 1, 11017, calendar.Wednesday)
		(10000, 1, 1, 2932897, calendar.Saturday)
		(1970, 1, 1, 0, calendar.Thursday)
		(1969, 12, 31, -1, calendar.Wednesday)
	]
	list.each(cases, (item) {
		(year, month, day, number, expected_weekday) = item
		date = valid_date(year, month, day)
		assert(calendar.day_number(date) == number)
		assert(calendar.from_day_number(number) == date)
		assert(calendar.weekday(date) == expected_weekday)
	})
	assert(calendar.weekday(calendar.from_day_number(-4)) == calendar.Sunday)
	assert(
		calendar.days_between(
			valid_date(1582, 10, 4)
			valid_date(1582, 10, 5)
		) ==
			1
	)
	Nil
}

// Independent January-based oracle from the review. The production algorithm
// instead decomposes March-based 400-year eras (note §4.3). C3 requires floor
// division; adjust the remainder to nonnegative here before dividing.
oracle_floor = (a: Int, b: Int) -> Int { { a - { a % b + b } % b } / b }

before_year = (y: Int) -> Int {
	365 * y +
		oracle_floor(y + 3, 4) -
		oracle_floor(y + 99, 100) +
		oracle_floor(y + 399, 400)
}

oracle_day = (year: Int, month: Int, day: Int) -> Int {
	before_month = month |> {
		1  -> 0
		2  -> 31
		3  -> 59
		4  -> 90
		5  -> 120
		6  -> 151
		7  -> 181
		8  -> 212
		9  -> 243
		10 -> 273
		11 -> 304
		_  -> 334
	}
	leap = year % 400 == 0 || { year % 4 == 0 && year % 100 != 0 }
	extra = { month > 2 && leap } |> {
		True  -> 1
		False -> 0
	}
	before_year(year) - before_year(1970) + before_month + extra + day - 1
}

// C3: both inverse laws, independent reference, negative quotients and all
// century/era boundaries in this range. Equality alone of two inverses would
// miss a common shift; oracle_day establishes the required epoch separately.
@test
calendar_day_laws = () {
	list.each(list.range(-800, 800), (year) {
		list.each(list.range(1, 12), (month) {
			length = calendar.days_in_month(year, month) |> {
				Ok(n)    -> n
				Error(_) -> panic("test month")
			}
			list.each([1, length], (day) {
				value = valid_date(year, month, day)
				expected = oracle_day(year, month, day)
				assert(calendar.day_number(value) == expected)
				assert(calendar.from_day_number(expected) == value)
			})
		})
	})
	list.each(list.range(-20, 20), (century) {
		center = oracle_day(century * 100, 3, 1)
		list.each(list.range(-3, 3), (delta) {
			day = center + delta
			value = calendar.from_day_number(day)
			assert(calendar.day_number(value) == day)
			(y, m, d) = calendar.date_parts(value)
			assert(oracle_day(y, m, d) == day)
		})
	})
	huge = int.power(10, 100) |> {
		Ok(n)    -> n
		Error(_) -> panic("test exponent")
	}
	list.each([-huge, huge, -719468, -719469, -719467, -1, 0, 1], (day) {
		list.each([-1, 0, 1], (delta) {
			n = day + delta
			value = calendar.from_day_number(n)
			assert(calendar.day_number(value) == n)
			(y, m, d) = calendar.date_parts(value)
			assert(oracle_day(y, m, d) == n)
			assert(calendar.date(y, m, d) == Ok(value))
		})
	})
	first = valid_date(-400, 2, 29)
	last = valid_date(2000, 3, 1)
	assert(calendar.add_days(first, calendar.days_between(first, last)) == last)
	assert(
		calendar.days_between(last, first) ==
			-calendar.days_between(first, last)
	)
	Nil
}

// C4: floor splitting for negative epoch nanoseconds and extreme offsets.
@test
calendar_civil_boundaries = () {
	epoch = time.from_unix_nanoseconds(0)
	before = calendar.at(time.from_unix_nanoseconds(-1), calendar.utc())
	assert(calendar.date_parts(before.date) == (1969, 12, 31))
	assert(calendar.time_parts(before.time) == (23, 59, 59, 999999999))
	plus = calendar.at(epoch, valid_offset(1439))
	assert(calendar.date_parts(plus.date) == (1970, 1, 1))
	assert(calendar.time_parts(plus.time) == (23, 59, 0, 0))
	minus = calendar.at(epoch, valid_offset(-1439))
	assert(calendar.date_parts(minus.date) == (1969, 12, 31))
	assert(calendar.time_parts(minus.time) == (0, 1, 0, 0))
	assert(
		calendar.time_parts(calendar.at(epoch, valid_offset(60)).time) ==
			(1, 0, 0, 0)
	)
	huge = int.power(10, 100) |> {
		Ok(n)    -> n
		Error(_) -> panic("test exponent")
	}
	list.each([-1439, -60, 0, 60, 1439], (minutes) {
		offset = valid_offset(minutes)
		list.each(
			[
				-huge
				-86400000000001
				-86400000000000
				-86399999999999
				-1
				0
				1
				86399999999999
				86400000000000
				huge
			]
			(n) {
				instant = time.from_unix_nanoseconds(n)
				civil = calendar.at(instant, offset)
				assert(calendar.to_instant(civil, offset) == instant)
				assert(
					calendar.at(calendar.to_instant(civil, offset), offset) ==
						civil
				)
				assert(
					calendar.parse(calendar.format(civil, offset)) ==
						Ok((civil, offset))
				)
			}
		)
	})
	Nil
}

// C5/C6: canonical expanded years, fractions, and zero offsets.
@test
calendar_text = () {
	cases = [
		("1970-01-01T00:00:00.1+00:00", "1970-01-01T00:00:00.1Z")
		("1970-01-01T00:00:00.000000001Z", "1970-01-01T00:00:00.000000001Z")
		("1970-01-01T00:00:00.123400000Z", "1970-01-01T00:00:00.1234Z")
		("1970-01-01T00:00:00.000000000Z", "1970-01-01T00:00:00Z")
		("0000-02-29T00:00:00Z", "0000-02-29T00:00:00Z")
		("-0001-01-01T00:00:00Z", "-0001-01-01T00:00:00Z")
		("-0400-02-29T00:00:00Z", "-0400-02-29T00:00:00Z")
		("-10000-01-01T00:00:00Z", "-10000-01-01T00:00:00Z")
		("+10000-01-01T00:00:00Z", "+10000-01-01T00:00:00Z")
		(
			"9999-12-31T23:59:59.999999999+23:59"
			"9999-12-31T23:59:59.999999999+23:59"
		)
		("0001-01-01T00:00:00-23:59", "0001-01-01T00:00:00-23:59")
	]
	list.each(cases, (pair) {
		(input, expected) = pair
		calendar.parse(input) |> {
			Error(_) -> panic("valid calendar text rejected")
			Ok((civil, offset)) -> {
				assert(calendar.format(civil, offset) == expected)
				assert(calendar.parse(expected) == Ok((civil, offset)))
			}
		}
	})
	// C5 requires every component constructor error to become BadText in parse.
	invalid = [
		""
		"1970-01-01"
		"1970-01-01T00:00:00"
		"1970-01-01T00:00:60Z"
		"1970-01-01T24:00:00Z"
		"1970-01-01T00:60:00Z"
		"1900-02-29T00:00:00Z"
		"1970-13-00T00:00:00Z"
		"1970-04-31T00:00:00Z"
		"1970-01-01T00:00:00-00:00"
		"1970-01-01T00:00:00+01:60"
		"1970-01-01T00:00:00+24:00"
		"1970-01-01T00:00:00-24:00"
		"1970-01-01T00:00:00+00:00:00"
		"1970-01-01T00:00:00.1234567890Z"
		"1970-01-01T00:00:00.Z"
		"1970-01-01T00:00:00.１Z"
		"-0000-01-01T00:00:00Z"
		"+0001-01-01T00:00:00Z"
		"10000-01-01T00:00:00Z"
		"+010000-01-01T00:00:00Z"
		"-00001-01-01T00:00:00Z"
		"-001-01-01T00:00:00Z"
		"197-01-01T00:00:00Z"
		"01970-01-01T00:00:00Z"
		"1970-1-01T00:00:00Z"
		"1970-01-1T00:00:00Z"
		"1970-01-01t00:00:00Z"
		"1970-01-01T00:00:00z"
		"1970-01-01 00:00:00Z"
		"1970-01-01T0:00:00Z"
		"1970-01-01T00:0:00Z"
		"1970-01-01T00:00:0Z"
		"1970-01-01T00:00:00Z "
		" 1970-01-01T00:00:00Z"
		"1970-01-01T00:00:00Z\u{0}"
		"１９７０-01-01T00:00:00Z"
	]
	list.each(invalid, (text) {
		assert(calendar.parse(text) == Error(calendar.BadText))
	})
	Nil
}

// C1 and spec §2.4 require constructor and constructor-update
// privacy. Run these negative checks with the exact lane compiler, restoring
// this owned file after each case. They are not deliberately failing @tests.
// direnv exec /home/agent/Programming/i python3 -c 'from pathlib import Path; exec("\n".join(x[4:] for x in Path("std/test/calendar_test.i").read_text().splitlines() if x.startswith("//| ")))'
//| import subprocess
//| from pathlib import Path
//| path = Path('std/test/calendar_test.i')
//| original = path.read_text()
//| cases = {
//|     'private Date constructor': 'bad = () { calendar.Date(2000, 13, 0) }',
//|     'private Date update': 'bad = (d: calendar.Date) { calendar.Date(..d) }',
//|     'private TimeOfDay constructor': 'bad = () { calendar.TimeOfDay(24, 0, 0, 0) }',
//|     'private Offset constructor': 'bad = () { calendar.Offset(1440) }',
//| }
//| baseline = subprocess.run(['target/release/i', 'check', '--project', 'std'], capture_output=True, text=True)
//| assert baseline.returncode == 0, baseline.stderr
//| try:
//|     for name, source in cases.items():
//|         path.write_text(original + '\n' + source + '\n')
//|         result = subprocess.run(['target/release/i', 'check', '--project', 'std'], capture_output=True, text=True)
//|         print(name, result.returncode, result.stderr, flush=True)
//|         assert result.returncode == 1 and 'error[E0204]' in result.stderr
//| finally:
//|     path.write_text(original)
