// spec: 9.4, 11.6
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

count = (n, acc) { n |> (0) { acc } | (_) { count(n - 1, acc + 1) } }

// 1000 frames that wait, then a loop of 1,000,000 tail calls after the
// resume.
down = (b, n) {
	n |>
		(0) {
			control.suspend(b, (k) { control.resume(k, Nil) })
			count(1000000, 0)
		}
		| (_) {
			x = down(b, n - 1)
			x + 1
		}
}

test("tail_calls_after_resume", () {
	check(
		show(control.boundary((b) { down(b, 1000) })),
		"1001000",
	)
})
