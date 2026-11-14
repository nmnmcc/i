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

Bad = (Int)

// `n` frames that wait, then a suspend whose `f` resumes at once, then an
// error.
down = (b, n) {
	n |>
		(0) {
			control.suspend(b, (k) { control.resume(k, Nil) })
			fail.raise(Bad(7))
		}
		| (_) {
			x = down(b, n - 1)
			x + 1
		}
}

// 25 frames, a `fail.catch`, then 25 frames under it.
mid = (b, n) {
	n |>
		(25) {
			r = fail.catch(() { down(b, 25) })
			r |> (Ok(x)) { x } | (Error(Bad(c))) { c * 100 }
		}
		| (_) {
			x = mid(b, n + 1)
			x + 1
		}
}

// After a resume at depth 50, the error of the innermost frame goes through
// the frames that wait, to the `fail.catch` near the boundary.
test("error_after_resume", () {
	r = control.boundary((b) { fail.catch(() { down(b, 50) }) })
	check(show(r), "Error(Bad(7))")
})

// The frames outside the `fail.catch` go on with its value.
test("caught_in_the_middle", () {
	check(
		show(control.boundary((b) { mid(b, 0) })),
		"725",
	)
})
