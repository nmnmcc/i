// spec: 9.4, 9.3
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/fail
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

More = (Continuation(Nil, Gen, Never, Never))
End = ()
Gen = More | End

// Not a tail call, so the frame stays.
keep = (t) { t |> (End) { End } | (t) { t } }

// One new frame per round.
g = (b, i) {
	control.suspend(b, (k) { More(k) })
	keep(g(b, i + 1))
}

// Resumes `n` times from a loop, then cancels.
drive = (step, n) {
	step |>
		(End) { Nil }
		| (More(k)) {
			n |>
				(0) { control.cancel(k) }
				| (_) { drive(control.resume(k, Nil), n - 1) }
		}
}

// After 10,000 rounds, the cancel goes through 10,000 frames that wait.
test("cancel_long_pending", () {
	check(
		show(
			drive(control.boundary((b) { g(b, 0) }), 10000),
		),
		"Nil",
	)
})

Step = (Continuation(Nil, Step, Int, Never))
Fin = ()
Step = Step | Fin

keep2 = (t) { t |> (Fin) { Fin } | (t) { t } }

// `n` frames, then a suspend; after the last of `left` resumes, an error.
deep = (b, n, left) {
	n |>
		(0) {
			control.suspend(b, (k) { Step(k) })
			left |> (0) { fail.raise(1) } | (_) { deep(b, 200, left - 1) }
		}
		| (_) { keep2(deep(b, n - 1, left)) }
}

run = (step) {
	step |> (Fin) { Fin } | (Step(k)) { run(control.resume(k, Nil)) }
}

// 50 steps of 200 frames each: the error goes through 10,000 frames that
// wait.
test("error_long_pending", () {
	r = fail.catch(() { run(control.boundary((b) { deep(b, 200, 50) })) })
	check(show(r), "Error(1)")
})
