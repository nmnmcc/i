(
	std/time
	std/list
	(Less, Equal, Greater) = std/order
)

// T1 requires every signed Int and exact unit multipliers.
@test
time_units = () {
	list.each(
		[-100000000000000000000, -2, -1, 0, 1, 2, 100000000000000000000]
		(n) {
			assert(time.to_nanoseconds(time.nanoseconds(n)) == n)
			assert(time.to_nanoseconds(time.milliseconds(n)) == n * 1000000)
			assert(time.to_nanoseconds(time.seconds(n)) == n * 1000000000)
			assert(time.to_nanoseconds(time.minutes(n)) == n * 60000000000)
			assert(time.to_nanoseconds(time.hours(n)) == n * 3600000000000)
			assert(time.minutes(n) == time.seconds(n * 60))
			assert(time.hours(n) == time.minutes(n * 60))
		}
	)
	assert(time.to_nanoseconds(time.seconds(-2)) == -2000000000)
	Nil
}

// T2 explicitly uses truncating division, for either sign of the divisor.
@test
time_arithmetic = () {
	assert(time.divide(time.nanoseconds(-5), 2) == Ok(time.nanoseconds(-2)))
	assert(time.divide(time.nanoseconds(5), -2) == Ok(time.nanoseconds(-2)))
	assert(time.divide(time.nanoseconds(-5), -2) == Ok(time.nanoseconds(2)))
	assert(time.divide(time.nanoseconds(5), 2) == Ok(time.nanoseconds(2)))
	assert(time.divide(time.seconds(1), 0) == Error(time.DivisionByZero))
	assert(time.divide(time.nanoseconds(0), 0) == Error(time.DivisionByZero))
	assert(time.divide(time.nanoseconds(0), -2) == Ok(time.nanoseconds(0)))
	assert(time.scale(time.nanoseconds(-2), 2) != time.nanoseconds(-5))
	list.each([-7, -1, 0, 1, 7], (a) {
		list.each([-9, -1, 0, 1, 9], (b) {
			d = time.nanoseconds(a)
			e = time.nanoseconds(b)
			assert(time.to_nanoseconds(time.add(d, e)) == a + b)
			assert(time.to_nanoseconds(time.subtract(d, e)) == a - b)
			assert(time.to_nanoseconds(time.scale(d, b)) == a * b)
			// §3.3's duration arithmetic law.
			assert(time.add(d, time.subtract(e, d)) == e)
		})
	})
	assert(time.compare(time.nanoseconds(-1), time.nanoseconds(0)) == Less)
	assert(time.compare(time.seconds(1), time.milliseconds(1000)) == Equal)
	assert(time.compare(time.seconds(1), time.nanoseconds(1)) == Greater)
	Nil
}

// T3's epoch and inverse laws apply to negative and arbitrarily large counts.
@test
time_instants = () {
	epoch = time.from_unix_nanoseconds(0)
	assert(
		time.shift(time.from_unix_nanoseconds(-1), time.nanoseconds(1)) == epoch
	)
	assert(
		time.between(
			time.from_unix_nanoseconds(7)
			time.from_unix_nanoseconds(2)
		) ==
			time.nanoseconds(-5)
	)
	list.each([-100000000000000000000, -1, 0, 1, 100000000000000000000], (a) {
		start = time.from_unix_nanoseconds(a)
		assert(time.unix_nanoseconds(start) == a)
		assert(
			time.from_unix_nanoseconds(time.unix_nanoseconds(start)) == start
		)
		list.each([-100, -1, 0, 1, 100], (b) {
			end = time.from_unix_nanoseconds(b)
			duration = time.nanoseconds(b)
			assert(time.shift(start, time.between(start, end)) == end)
			assert(time.between(start, time.shift(start, duration)) == duration)
		})
	})
	assert(time.compare_instants(time.from_unix_nanoseconds(-1), epoch) == Less)
	assert(time.compare_instants(epoch, time.from_unix_nanoseconds(0)) == Equal)
	assert(
		time.compare_instants(time.from_unix_nanoseconds(1), epoch) == Greater
	)
	Nil
}

// §3.4 specifically requires hours(10^100), and spec §11.2 forbids overflow.
@test
time_huge_counts = () {
	huge = 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
	duration = time.hours(huge)
	start = time.from_unix_nanoseconds(-1)
	assert(time.to_nanoseconds(duration) == huge * 3600000000000)
	assert(
		time.shift(time.shift(start, duration), time.scale(duration, -1)) ==
			start
	)
	assert(time.between(start, time.shift(start, duration)) == duration)
	assert(
		time.to_nanoseconds(
			time.add(time.nanoseconds(9223372036854775807), time.nanoseconds(1))
		) ==
			9223372036854775808
	)
	assert(
		time.to_nanoseconds(
			time.scale(time.nanoseconds(-9223372036854775808), -1)
		) ==
			9223372036854775808
	)
	Nil
}

// T1/T3 nominal separation and §3.2 hidden constructors. These negative cases
// are temporarily appended to this owned file; each must fail checking, and
// the file is restored even if a check fails. Run from the lane with:
// direnv exec /home/agent/Programming/i python3 -c 'from pathlib import Path; exec("\n".join(x[4:] for x in Path("std/test/time_test.i").read_text().splitlines() if x.startswith("//| ")))'
//| import subprocess
//| from pathlib import Path
//| path = Path('std/test/time_test.i')
//| original = path.read_text()
//| compiler = 'target/release/i'
//| cases = {
//|     'Instant cannot be added as a Duration (T1/T3)': ('E0501', 'bad = () { time.add(time.from_unix_nanoseconds(0), time.nanoseconds(1)) }'),
//|     'Duration constructor is private (§3.2)': ('E0204', 'bad = () { time.Duration(0) }'),
//|     'Instant constructor is private (§3.2)': ('E0204', 'bad = () { time.Instant(0) }'),
//| }
//| baseline = subprocess.run([compiler, 'check', '--project', 'std'], capture_output=True, text=True)
//| assert baseline.returncode == 0, baseline.stderr
//| try:
//|     for name, (code, source) in cases.items():
//|         path.write_text(original + '\n' + source + '\n')
//|         result = subprocess.run([compiler, 'check', '--project', 'std'], capture_output=True, text=True)
//|         print(name, result.returncode, result.stderr, flush=True)
//|         assert result.returncode == 1 and f'error[{code}]' in result.stderr
//| finally:
//|     path.write_text(original)
