// spec: 9.4, 9.3, 9.5
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/fail
	i/resource
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

// The final function runs for an error after a resume, and suspends. When
// it is resumed and ends, the error goes on to `fail.catch`.
test("final_suspends_during_an_error", () {
	r = control.boundary((outer) {
		control.boundary((b) {
			fail.catch(() {
				resource.ensure(
					() {
						control.suspend(b, (k) { control.resume(k, Nil) })
						fail.raise(1)
					},
					() {
						control.suspend(outer, (k2) { control.resume(k2, Nil) })
					},
				)
			})
		})
	})
	check(show(r), "Error(1)")
})
