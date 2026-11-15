(
	Clock(..)
	now
)

Clock = (now = () -> Int)
now = () <- (clock : Clock) { clock.now() }
