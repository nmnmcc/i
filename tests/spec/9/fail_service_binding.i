// spec: 6.8, 9.2, 9.3
// expect: pass

// EF13 / T21: Fail binds the same service value that provision installed.
(
	i/control
	i/fail
)

raise = () <- (h : Fail(String)) { h.raise("x") }

main = () {
	answer = control.boundary((b) {
		raise() <- (Fail(raise = (s : String) { control.escape(b, s) }))
	})
	assert(answer == "x")
	assert(fail.catch(() { raise() }) == Error("x"))
}
