//// Clock observations and scheduler-friendly sleeps.
(
	i/request
	i/fail
)

(
	Clock(..)
	ClockError(..)
	monotonic_ns
	unix_ns
	sleep_ns
)

ClockError = (operation = String, code = String)

@host("i:clock/v1", "request")
Clock = (
	monotonic_ns = () -> Request(Result(Int, String)),
	unix_ns = () -> Request(Result(Int, String)),
	sleep_ns = (Int) -> Request(Result(Nil, String)),
)

monotonic_ns = () <- (clock : Clock) {
	request.await_owned(clock.monotonic_ns()) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ClockError(operation = "monotonic_ns", code = code))
		}
}

unix_ns = () <- (clock : Clock) {
	request.await_owned(clock.unix_ns()) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ClockError(operation = "unix_ns", code = code))
		}
}

sleep_ns = (duration : Int) <- (clock : Clock) {
	request.await_owned(clock.sleep_ns(duration)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ClockError(operation = "sleep_ns", code = code))
		}
}
