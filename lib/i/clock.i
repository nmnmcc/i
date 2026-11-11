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

ClockError {
	ClockError(operation: String, code: String)
}

@host("i:clock/v1", "request")
Clock {
	Clock(
		monotonic_ns: () -> Request(Result(Int, String))
		unix_ns:      () -> Request(Result(Int, String))
		sleep_ns:     (duration: Int) -> Request(Result(Nil, String))
	)
}

monotonic_ns = (clock ! Clock) {
	request.await_owned(clock.monotonic_ns()) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(ClockError("monotonic_ns", code))
	}
}

unix_ns = (clock ! Clock) {
	request.await_owned(clock.unix_ns()) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(ClockError("unix_ns", code))
	}
}

sleep_ns = (duration: Int, clock ! Clock) {
	request.await_owned(clock.sleep_ns(duration)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(ClockError("sleep_ns", code))
	}
}
