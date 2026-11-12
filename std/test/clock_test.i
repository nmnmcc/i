(
	testlib = (test) = std/test
	i/clock
	i/request
	i/fail
	std/time
	std/calendar
	std/log
	(Some) = std/option
)

test("clock_fake", () {
	! clock.Clock = clock.Clock(
		() { request.ready(Ok(42)) }
		() { request.ready(Ok(-7)) }
		(n) {
			request.ready({ n < 0 } |> {
				True  -> Error("invalid")
				False -> Ok(Nil)
			})
		}
	)
	assert(clock.monotonic_ns() == 42)
	assert(clock.unix_ns() == -7)
	clock.sleep_ns(0)
	assert(time.unix_nanoseconds(time.now()) == -7)
	assert(calendar.format_now() == "1969-12-31T23:59:59.999999993Z")
	entry = log.timestamped(
		log.Entry(log.Info, "clock", [], Some(time.from_unix_nanoseconds(0)))
	)
	assert(entry.at == Some(time.from_unix_nanoseconds(-7)))
	time.sleep(time.nanoseconds(0))
	assert(time.measure(() { 12 }) == (12, time.nanoseconds(0)))
	assert(
		fail.catch(() { time.sleep(time.nanoseconds(-1)) }) ==
			Error(clock.ClockError("sleep_ns", "invalid"))
	)
	assert(
		fail.catch(() { clock.sleep_ns(-1) }) ==
			Error(clock.ClockError("sleep_ns", "invalid"))
	)
	Nil
})
