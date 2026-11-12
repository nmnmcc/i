//// Proleptic Gregorian dates for all integer years, and explicit fixed offsets.
//// Date, TimeOfDay and Offset have private constructors; Civil, Weekday and
//// CalendarError expose their constructors. All validation returns Result.
////
//// C1: leap years divide by 4, except centuries not divisible by 400. Year 0
//// is 1 BCE. Months are 1..12; validate month before day, with no 1582 gap.
//// C2: validate hour 0..23, minute/second 0..59 and nanosecond 0..999999999
//// in that order. Offsets are whole minutes east of UTC in -1439..1439.
//// C3: day zero is 1970-01-01. Eras and signed days use floor division.
//// C4: at adds the offset before splitting; to_instant subtracts it. Weekdays
//// run Monday through Sunday; day zero is Thursday. No leap seconds or zones.
//// C5: parse accepts YYYY-MM-DDTHH:mm:ss, optional 1..9 fractional digits,
//// then Z or a signed HH:mm offset. Years outside 0..9999 use a sign;
//// negative magnitudes have at least four digits, with no excess zeroes.
//// Reject negative zero years, -00:00 offsets and non-ASCII syntax. Every
//// parse failure is BadText; component constructors retain precise errors.
//// C6: format uses canonical years, two-digit fields, trimmed fractional
//// nanoseconds and Z for zero offset. No locale or host service is consulted.
////
//// Laws: from_day_number(day_number(d)) = d; day_number(from_day_number(n))
//// = n; at(to_instant(c,o),o) = c; to_instant(at(t,o),o) = t;
//// parse(format(c,o)) = Ok((c,o)); add_days(d,days_between(d,e)) = e.
////
//// Public values:
//// Date { Date(Int, Int, Int) }, TimeOfDay { TimeOfDay(Int, Int, Int, Int) }
//// and Offset { Offset(Int) } keep their constructors private.
//// Civil { Civil(date: Date, time: TimeOfDay) }
//// Weekday { Monday Tuesday Wednesday Thursday Friday Saturday Sunday }
//// CalendarError { BadMonth BadDay BadHour BadMinute BadSecond
////                 BadNanosecond BadOffset BadText }
////
//// date: (year: Int, month: Int, day: Int) -> Result(Date, CalendarError)
//// time_of_day: (hour: Int, minute: Int, second: Int, nanosecond: Int)
////              -> Result(TimeOfDay, CalendarError)
//// offset: (minutes: Int) -> Result(Offset, CalendarError)
//// utc: () -> Offset
//// date_parts: (date: Date) -> (Int, Int, Int)
//// time_parts: (time: TimeOfDay) -> (Int, Int, Int, Int)
//// offset_minutes: (offset: Offset) -> Int
//// is_leap_year: (year: Int) -> Bool
//// days_in_month: (year: Int, month: Int) -> Result(Int, CalendarError)
//// day_number: (date: Date) -> Int
//// from_day_number: (day: Int) -> Date
//// weekday: (date: Date) -> Weekday
//// add_days: (date: Date, days: Int) -> Date
//// days_between: (start: Date, end: Date) -> Int
//// at: (instant: time.Instant, offset: Offset) -> Civil
//// to_instant: (civil: Civil, offset: Offset) -> time.Instant
//// parse: (text: String) -> Result((Civil, Offset), CalendarError)
//// format: (civil: Civil, offset: Offset) -> String
////
//// No month/year addition policy is implicit. Conversion uses a fixed number
//// of exact integer operations. Text scanning and assembly are linear apart
//// from integer digit arithmetic; no suffix string is repeatedly rebuilt.
(
	i/core
	i/fail
	std/int
	std/list
	std/string
	std/time
)

(
	Date
	TimeOfDay
	Offset
	Civil(..)
	Weekday(..)
	CalendarError(..)
	date
	time_of_day
	offset
	utc
	date_parts
	time_parts
	offset_minutes
	is_leap_year
	days_in_month
	day_number
	from_day_number
	weekday
	add_days
	days_between
	at
	to_instant
	parse
	format
	now
	format_now
)

Date = (Int, Int, Int)

TimeOfDay = (Int, Int, Int, Int)

Offset = (Int)

Civil = (date = Date, time = TimeOfDay)

Monday = ()
Tuesday = ()
Wednesday = ()
Thursday = ()
Friday = ()
Saturday = ()
Sunday = ()
Weekday = Monday | Tuesday | Wednesday | Thursday | Friday | Saturday | Sunday

BadMonth = ()
BadDay = ()
BadHour = ()
BadMinute = ()
BadSecond = ()
BadNanosecond = ()
BadOffset = ()
BadText = ()
CalendarError = BadMonth
	| BadDay
	| BadHour
	| BadMinute
	| BadSecond
	| BadNanosecond
	| BadOffset
	| BadText

is_leap_year = (year: Int) -> Bool {
	year % 4 == 0 && { year % 100 != 0 || year % 400 == 0 }
}

days_in_month = (year: Int, month: Int) -> Result(Int, CalendarError) {
	month |>
		(1 | 3 | 5 | 7 | 8 | 10 | 12) { Ok(31) }
		| (4 | 6 | 9 | 11) { Ok(30) }
		| (2) { Ok({ is_leap_year(year) } |> (True) { 29 } | (False) { 28 }) }
		| (_) { Error(BadMonth) }
}

date = (year: Int, month: Int, day: Int) -> Result(Date, CalendarError) {
	days_in_month(year, month) |>
		(Error(e)) { Error(e) }
		| (Ok(count)) {
			{ day >= 1 && day <= count } |>
				(True) { Ok(Date(year, month, day)) }
				| (False) { Error(BadDay) }
		}
}

time_of_day = (hour: Int, minute: Int, second: Int, nanosecond: Int) -> Result(
	TimeOfDay,
	CalendarError,
) {
	{ hour < 0 || hour > 23 } |>
		(True) { Error(BadHour) }
		| (False) {
			{ minute < 0 || minute > 59 } |>
				(True) { Error(BadMinute) }
				| (False) {
					{ second < 0 || second > 59 } |>
						(True) { Error(BadSecond) }
						| (False) {
							{ nanosecond < 0 || nanosecond > 999999999 } |>
								(True) { Error(BadNanosecond) }
								| (False) {
									Ok(
										TimeOfDay(
											hour,
											minute,
											second,
											nanosecond,
										),
									)
								}
						}
				}
		}
}

offset = (minutes: Int) -> Result(Offset, CalendarError) {
	{ minutes >= -1439 && minutes <= 1439 } |>
		(True) { Ok(Offset(minutes)) }
		| (False) { Error(BadOffset) }
}

utc = () -> Offset { Offset(0) }

date_parts = (date: Date) -> (Int, Int, Int) {
	Date(y, m, d) = date
	(y, m, d)
}

time_parts = (time: TimeOfDay) -> (Int, Int, Int, Int) {
	TimeOfDay(h, m, s, n) = time
	(h, m, s, n)
}

offset_minutes = (offset: Offset) -> Int {
	Offset(minutes) = offset
	minutes
}

// Only positive divisors occur in the calendar algorithms (C3).
floor_divide = (a: Int, b: Int) -> Int {
	{ a % b < 0 } |> (True) { a / b - 1 } | (False) { a / b }
}

modulo = (a: Int, b: Int) -> Int { a - b * floor_divide(a, b) }

day_number = (date: Date) -> Int {
	Date(y, m, d) = date
	y1 = { m <= 2 } |> (True) { y - 1 } | (False) { y }
	era = floor_divide(y1, 400)
	yoe = y1 - era * 400
	mp = { m > 2 } |> (True) { m - 3 } | (False) { m + 9 }
	doy = floor_divide(153 * mp + 2, 5) + d - 1
	doe = 365 * yoe + floor_divide(yoe, 4) - floor_divide(yoe, 100) + doy
	era * 146097 + doe - 719468
}

from_day_number = (day: Int) -> Date {
	z = day + 719468
	era = floor_divide(z, 146097)
	doe = z - era * 146097
	yoe = floor_divide(
		doe -
			floor_divide(doe, 1460) +
			floor_divide(doe, 36524) -
			floor_divide(doe, 146096),
		365,
	)
	y = yoe + era * 400
	doy = doe - { 365 * yoe + floor_divide(yoe, 4) - floor_divide(yoe, 100) }
	mp = floor_divide(5 * doy + 2, 153)
	d = doy - floor_divide(153 * mp + 2, 5) + 1
	m = { mp < 10 } |> (True) { mp + 3 } | (False) { mp - 9 }
	Date({ m <= 2 } |> (True) { y + 1 } | (False) { y }, m, d)
}

weekday = (date: Date) -> Weekday {
	modulo(day_number(date) + 3, 7) |>
		(0) { Monday }
		| (1) { Tuesday }
		| (2) { Wednesday }
		| (3) { Thursday }
		| (4) { Friday }
		| (5) { Saturday }
		| (_) { Sunday }
}

add_days = (date: Date, days: Int) -> Date {
	from_day_number(day_number(date) + days)
}

days_between = (start: Date, end: Date) -> Int {
	day_number(end) - day_number(start)
}

at = (instant: time.Instant, offset: Offset) -> Civil {
	local = time.unix_nanoseconds(instant) +
		offset_minutes(offset) * 60000000000
	day = floor_divide(local, 86400000000000)
	rest = local - day * 86400000000000
	Civil(
		date = from_day_number(day),
		time = TimeOfDay(
			rest / 3600000000000,
			rest / 60000000000 % 60,
			rest / 1000000000 % 60,
			rest % 1000000000,
		),
	)
}

to_instant = (civil: Civil, offset: Offset) -> time.Instant {
	d = civil.date
	t = civil.time
	TimeOfDay(h, m, s, n) = t
	time.from_unix_nanoseconds(
		day_number(d) * 86400000000000 +
			h * 3600000000000 +
			m * 60000000000 +
			s * 1000000000 +
			n -
			offset_minutes(offset) * 60000000000,
	)
}

// Every input check in the text profile maps to BadText, including constructor
// failures. These helpers raise only within parse's enclosing pure catch.
require_text = (valid: Bool) -> Nil {
	valid |> (True) { Nil } | (False) { fail.raise(BadText) }
}

expect = (chars: List(Int), wanted: Int) -> List(Int) {
	chars |>
		([c, ..tail]) {
			{ c == wanted } |> (True) { tail } | (False) { fail.raise(BadText) }
		}
		| ([]) { fail.raise(BadText) }
}

digits = (chars: List(Int), count: Int, value: Int) -> (Int, List(Int)) {
	{ count == 0 } |>
		(True) { (value, chars) }
		| (False) {
			chars |>
				([c, ..tail]) {
					require_text(c >= 48 && c <= 57)
					digits(tail, count - 1, value * 10 + c - 48)
				}
				| ([]) { fail.raise(BadText) }
		}
}

scan_digits = (chars: List(Int), value: Int, count: Int) -> (
	Int,
	Int,
	List(Int),
) {
	chars |>
		([c, ..tail]) {
			{ c >= 48 && c <= 57 } |>
				(True) { scan_digits(tail, value * 10 + c - 48, count + 1) }
				| (False) { (value, count, chars) }
		}
		| ([]) { (value, count, []) }
}

read_year = (chars: List(Int)) -> (Int, List(Int)) {
	(sign, rest) = chars |>
		([43, ..tail]) { (1, tail) }
		| ([45, ..tail]) { (-1, tail) }
		| (_) { (0, chars) }
	(magnitude, count, tail) = scan_digits(rest, 0, 0)
	leading_zero = rest |> ([48, .._]) { True } | (_) { False }
	sign |>
		(0) { require_text(count == 4) }
		| (1) { require_text(magnitude > 9999 && leading_zero == False) }
		| (_) {
			require_text(
				magnitude != 0 &&
					count >= 4 &&
					{ count == 4 || leading_zero == False },
			)
		}
	year = { sign < 0 } |> (True) { -magnitude } | (False) { magnitude }
	(year, expect(tail, 45))
}

power_of_ten = (exponent: Int) -> Int {
	{ exponent == 0 } |>
		(True) { 1 }
		| (False) { 10 * power_of_ten(exponent - 1) }
}

read_fraction = (chars: List(Int)) -> (Int, List(Int)) {
	chars |>
		([46, ..tail]) {
			(n, count, rest) = scan_digits(tail, 0, 0)
			require_text(count >= 1 && count <= 9)
			(n * power_of_ten(9 - count), rest)
		}
		| (_) { (0, chars) }
}

read_offset = (chars: List(Int)) -> Offset {
	chars |>
		([90]) { utc() }
		| ([sign, ..tail]) {
			require_text(sign == 43 || sign == 45)
			(h, r1) = digits(tail, 2, 0)
			(m, r2) = digits(expect(r1, 58), 2, 0)
			require_text(r2 == [] && h <= 23 && m <= 59)
			magnitude = h * 60 + m
			require_text(sign != 45 || magnitude != 0)
			Offset(
				{ sign == 45 } |> (True) { -magnitude } | (False) { magnitude },
			)
		}
		| ([]) { fail.raise(BadText) }
}

parse = (text: String) -> Result((Civil, Offset), CalendarError) {
	fail.catch(() {
		chars = list.map(core.codepoints(text), core.codepoint_to_int)
		(y, r1) = read_year(chars)
		(m, r2) = digits(r1, 2, 0)
		(d, r3) = digits(expect(r2, 45), 2, 0)
		(h, r4) = digits(expect(r3, 84), 2, 0)
		(minute, r5) = digits(expect(r4, 58), 2, 0)
		(s, r6) = digits(expect(r5, 58), 2, 0)
		(n, r7) = read_fraction(r6)
		off = read_offset(r7)
		day = date(y, m, d) |>
			(Ok(value)) { value }
			| (Error(_)) { fail.raise(BadText) }
		tod = time_of_day(h, minute, s, n) |>
			(Ok(value)) { value }
			| (Error(_)) { fail.raise(BadText) }
		(Civil(date = day, time = tod), off)
	})
}

padded = (value: Int, width: Int) -> String {
	string.pad_left(int.to_string(value), width, "0")
}

year_text = (year: Int) -> String {
	{ year < 0 } |>
		(True) { "-" <> padded(-year, 4) }
		| (False) {
			{ year > 9999 } |>
				(True) { "+" <> int.to_string(year) }
				| (False) { padded(year, 4) }
		}
}

fraction_text = (n: Int) -> String {
	trim = (value: Int, width: Int) -> String {
		{ value % 10 == 0 } |>
			(True) { trim(value / 10, width - 1) }
			| (False) { "." <> padded(value, width) }
	}
	{ n == 0 } |> (True) { "" } | (False) { trim(n, 9) }
}

offset_text = (offset: Offset) -> String {
	minutes = offset_minutes(offset)
	{ minutes == 0 } |>
		(True) { "Z" }
		| (False) {
			magnitude = int.abs(minutes)
			string.concat(
				[
					{ minutes < 0 } |> (True) { "-" } | (False) { "+" },
					padded(magnitude / 60, 2),
					":",
					padded(magnitude % 60, 2),
				],
			)
		}
}

format = (civil: Civil, offset: Offset) -> String {
	d = civil.date
	t = civil.time
	Date(y, m, day) = d
	TimeOfDay(h, minute, s, n) = t
	string.concat(
		[
			year_text(y),
			"-",
			padded(m, 2),
			"-",
			padded(day, 2),
			"T",
			padded(h, 2),
			":",
			padded(minute, 2),
			":",
			padded(s, 2),
			fraction_text(n),
			offset_text(offset),
		],
	)
}

/// Reads the current civil time in UTC.
now = () { at(time.now(), utc()) }

/// Formats the current UTC time.
format_now = () { format(now(), utc()) }
