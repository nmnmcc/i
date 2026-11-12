@host("app:clock/v1")
Clock = (now = () -> Int)
now = () <- (clock: Clock) { clock.now() }

main = () { assert(now() == 42) <- (Clock(now = () { 42 })) }
