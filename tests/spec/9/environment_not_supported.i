// spec: 9.2
// expect: compile-error E0701
// Environments come in a later version of the compiler.

Clock { Clock(now: () -> Int) }

elapsed = (start: Int, clock ! Clock) { clock.now() - start }

@test
fixed = () {
  ! Clock = Clock(now: () { 42 })
  case elapsed(40) { 2 -> Nil  _ -> panic("no") }
}
