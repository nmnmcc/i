@host("app:clock/v1")
Clock { Clock(now: () -> Int) }
now = (clock ! Clock) { clock.now() }

main = () {
  ! Clock = Clock(now: () { 42 })
  assert(now() == 42)
}
