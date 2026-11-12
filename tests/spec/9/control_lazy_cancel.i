// spec: 9.4, 9.5
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/resource
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

Paused = (Continuation(Nil, Paused, Never, Never))
Done = ()
Paused = Paused | Done

Held = (Continuation(Nil, Held, Never, Never))
Over = ()
Held = Held | Over

// Puts `text` in front of the value of the boundary `log`.
note = (log, text) {
	control.suspend(log, (k) { text <> " " <> control.resume(k, Nil) })
}

// 100 frames, with a `resource.ensure` at depths 10, 50 and 90. The final
// function at depth 50 suspends to `hold` in the middle.
layer = (log, hold, b, n) {
	n |>
		(0) {
			control.suspend(b, (k) { Paused(k) })
			0
		}
		| (10) { guarded(log, hold, b, n, () { note(log, "final 10") }) }
		| (50) {
			guarded(log, hold, b, n, () {
				note(log, "final 50")
				control.suspend(hold, (k2) { Held(k2) })
				note(log, "final 50 again")
			})
		}
		| (90) { guarded(log, hold, b, n, () { note(log, "final 90") }) }
		| (_) {
			x = layer(log, hold, b, n - 1)
			x + 1
		}
}

guarded = (log, hold, b, n, with) {
	x = resource.ensure(() { layer(log, hold, b, n - 1) }, with)
	x + 1
}

// A cancel at depth 100 runs the final functions innermost first. One of
// them suspends, and the cancel goes on when it is resumed later.
test("cancel_deep", () {
	r = control.boundary((log) {
		held = control.boundary((hold) {
			p = control.boundary((b) {
				layer(log, hold, b, 100)
				Done
			})
			p |>
				(Paused(k)) { control.cancel(k) }
				| (Done) { panic("not paused") }
			note(log, "cancelled")
			Over
		})
		held |>
			(Held(k2)) {
				note(log, "resume")
				{ control.resume(k2, Nil) } |>
					(Over) { "end" }
					| (Held(_)) { "held again" }
			}
			| (Over) { "not held" }
	})
	check(r, "final 10 final 50 resume final 50 again final 90 cancelled end")
})
