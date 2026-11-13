// spec: 9.4, 9.5, 11.7
// expect: stop "control: boundary is not active"
// at: spec/main:56:4
(
	i/control
	i/core
	i/resource
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

Paused(B) = (Continuation(Nil, Paused(B), Never, Never), B)
Done = ()
Paused(B) = Paused(B) | Done

main = () {
	// A cancel goes through `inner`, which is paused inside `k`. The final
	// function inside it runs, and suspends to `top`.
	r = control.boundary((top) {
		control.boundary((outer) {
			control.boundary((inner) {
				resource.ensure(
					() {
						control.suspend(outer, (k) {
							control.cancel(k)
							"c"
						})
					},
					() {
						control.suspend(top, (k) {
							"final, " <> control.resume(k, Nil)
						})
					},
				)
			})
		})
	})
	check(r, "final, c")
	// After the cancel, `inner` is not active.
	p = control.boundary((outer) {
		control.boundary((inner) {
			control.suspend(outer, (k) { Paused(k, inner) })
			0
		})
		Done
	})
	p |>
		(Paused(k, inner)) {
			control.cancel(k)
			control.suspend(inner, (k2) { 1 })
		}
		| (Done) { panic("not paused") }
	Nil
}
