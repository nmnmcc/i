//// Clock observations and scheduler-friendly sleeps.
(
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

@import("i:clock/v1")
Clock = (
	monotonic_ns = () -> Result(Int, String),
	unix_ns = () -> Result(Int, String),
	sleep_ns = (Int) -> Result((), String),
)

monotonic_ns = () <- (clock : Clock) {
	clock.monotonic_ns() |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(ClockError(operation = "monotonic_ns", code = code))
		}
}

unix_ns = () <- (clock : Clock) {
	clock.unix_ns() |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(ClockError(operation = "unix_ns", code = code))
		}
}

sleep_ns = (duration : Int) <- (clock : Clock) {
	clock.sleep_ns(duration) |>
		(Ok(value)) { value }
		& (Error(code)) {
			fail.raise(ClockError(operation = "sleep_ns", code = code))
		}
}
