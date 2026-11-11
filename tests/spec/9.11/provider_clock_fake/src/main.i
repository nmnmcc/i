(i/clock, i/request)
fake = () { clock.Clock(
  () { request.ready(Ok(42)) }
  () { request.ready(Ok(42)) }
  (_duration: Int) { request.ready(Ok(Nil)) }
) }
