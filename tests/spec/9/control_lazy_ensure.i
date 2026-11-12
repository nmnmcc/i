// spec: 9.4, 9.3, 9.5
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/fail
	i/resource
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

Bad = (Int)

Paused = (Continuation(Nil, Paused, Never, Never))
Done = ()
Paused = Paused | Done

// Puts `text` in front of the value of the boundary `log`.
note = (log, text) {
	control.suspend(log, (k) { text <> " " <> control.resume(k, Nil) })
}

// `n` frames that wait.
wait = (n, f) {
	n |>
		(0) { f() }
		| (_) {
			x = wait(n - 1, f)
			x + 1
		}
}

// A `resource.ensure` at each of `n` depths, with frames between them,
// then `last`.
layer = (log, n, last) {
	n |>
		(0) { last() }
		| (_) {
			x = resource.ensure(
				() { wait(3, () { layer(log, n - 1, last) }) },
				() { note(log, "final " <> show(n)) },
			)
			x + 1
		}
}

// An error after a resume runs the final functions innermost first.
test("error_runs_finals_innermost_first", () {
	r = control.boundary((log) {
		e = fail.catch(() {
			control.boundary((b) {
				layer(log, 3, () {
					control.suspend(b, (k) { control.resume(k, Nil) })
					fail.raise(Bad(1))
				})
			})
		})
		show(e)
	})
	check(r, "final 1 final 2 final 3 Error(Bad(1))")
})

// So does a cancel.
test("cancel_runs_finals_innermost_first", () {
	r = control.boundary((log) {
		p = control.boundary((b) {
			layer(log, 3, () {
				control.suspend(b, (k) { Paused(k) })
				0
			})
			Done
		})
		p |> (Paused(k)) { control.cancel(k) } | (Done) { panic("not paused") }
		"cancelled"
	})
	check(r, "final 1 final 2 final 3 cancelled")
})
