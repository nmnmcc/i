(i/clock)
@test
fake_time = () { assert(clock.monotonic_ns() == 42) }
