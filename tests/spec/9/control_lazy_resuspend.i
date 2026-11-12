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

More = (String, Continuation(String, Step, Never, Never))
Done = (String)
Step = More | Done

// Puts `text` in front of the value of the boundary `log`.
note = (log, text) {
	control.suspend(log, (k) { text <> " " <> control.resume(k, Nil) })
}

// Each frame puts its number in the value once. The innermost suspends
// twice; frame 3 has a `resource.ensure` around the frames inside it.
gen = (log, b, n) {
	n |>
		(0) {
			a = control.suspend(b, (k) { More("a", k) })
			c = control.suspend(b, (k) { More(a, k) })
			a <> c
		}
		| (3) {
			x = resource.ensure(
				() { gen(log, b, n - 1) },
				() { note(log, "final") },
			)
			show(n) <> x
		}
		| (_) {
			x = gen(log, b, n - 1)
			show(n) <> x
		}
}

// Resumes from a loop outside the boundary.
drive = (step, acc) {
	step |>
		(Done(s)) { acc <> "|" <> s }
		| (More(v, k)) { drive(control.resume(k, v <> "!"), acc <> v) }
}

// Resume, suspend, resume: every frame runs once, and the final function
// once, after the frames inside the `resource.ensure`.
test("resume_suspend_resume", () {
	r = control.boundary((log) {
		drive(control.boundary((b) { Done(gen(log, b, 5)) }), "")
	})
	check(r, "final aa!|54321a!a!!")
})
