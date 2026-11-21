// spec: 9.4, 9.7, 9.3
// expect: pass
// output: double 10
// output: double 20
// output: double -1
(
	i/control
	i/fail
)

@host("test:async/v1", "async")
Later = (double = (Int) <- (Fail(String)) -> Int)

Paused = (
	Continuation(
		(),
		Step,
		Fail(String) | Later,
	),
)
Done = (Int)
Step = Paused | Done

check = (ok) { ok |> (True()) { () } & (False()) { panic("async resume") } }

leaf = (b) <- (later : Later) {
	control.suspend(b, (k) { Paused(k) })
	a = later.double(10)
	c = later.double(a)
	error = fail.catch(() { later.double(-1) })
	error |> (Error("negative")) { () } & (_) { panic("async error was lost") }
	c
}

// These frames stay pending while leaf makes its two successful host
// calls and catches an error from a third (9.7: a normal call in i code).
down = (b, n) { n |> (0) { leaf(b) } & (_) { down(b, n - 1) + 1 } }

main = () <- (later : Later) {
	k = control.boundary((b) { Done(down(b, 10)) }) |>
		(Paused(k)) { k }
		& (Done(_)) { panic("not paused") }
	control.resume(k, ()) |>
		(Done(n)) { check(n == 50) }
		& (Paused(_)) { panic("paused twice") }
}
