// spec: 9.3, 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/main:21:4
// at: spec/main:20:34
(
	i/control
	i/failure
)

// A continuation made inside a `catch`, but not containing it, is resumed
// after the catch has ended: an escape to the catch from it stops.
Syn3ServiceFail(E) = Fail(E)

the_fail = () <- (f : Syn3ServiceFail(Int)) { f }

main = () {
	r = failure.catch(() {
		fail = the_fail()
		control.boundary((g) {
			control.suspend(g, (k) { () { control.resume(k, Nil)() } })
			fail.raise(1)
		})
	})
	r |> (Ok(later)) { later() } | (Error(_)) { Nil }
}
