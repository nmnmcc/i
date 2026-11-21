(
	Logger(..)
	Clock(..)
)

@import("app:log/v1")
Logger = (log = (String) <- (Fail(String)) -> ())

@import("app:clock/v1", "now")
Clock = (now = () -> Int)
