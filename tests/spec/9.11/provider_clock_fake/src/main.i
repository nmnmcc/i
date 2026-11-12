(i/clock, i/request)
fake = () { clock.Clock(
  monotonic_ns: () { request.ready(Ok(42)) }
  unix_ns: () { request.ready(Ok(42)) }
  sleep_ns: (_duration: Int) { request.ready(Ok(Nil)) }
) }
